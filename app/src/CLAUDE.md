# app/src — карта крейту + дерево рішень "куди класти новий код"

- Усе поза `#[cfg(feature = "ssr")]` мусить збиратися у WASM: без `sea-orm`/`tokio`/файлової системи.
- Нормалізація/парсинг/валідація — чисті функції з табличними тестами на граблях з `docs/source-analysis.md`.
- UI-тексти — українською.

## Дерево рішень "куди класти новий код" (docs/spec/07-code-structure.md §3)

1. Чиста функція без UI і БД (нормалізація, дати, валідація, підрахунки) → `domain/`
2. Тип, що передається між клієнтом і сервером → `types/`
3. SQL / SeaORM → `backend/repo/<агрегат>.rs`
4. Перевірка прав → `backend/policy.rs` (і тільки там)
5. `#[server]`-функція: для однієї сторінки → `pages/<p>/server.rs`; для ≥ 2 → `services/<домен>.rs`
6. UI-компонент, що не знає домену → `components/`
7. Доменний компонент: для однієї сторінки → `pages/<p>/components/`; для ≥ 2 → `widgets/`
8. Каркас сторінки (шапка, меню) → `layout/`
9. Реактивний хелпер → `hooks/`; глобальний контекст → `state/`
10. Нова сторінка → `pages/<p>/` + рядок у `routes.rs` + рядок у `pages/CLAUDE.md`

## Напрямок залежностей (§2.3, зверху вниз, ніколи навпаки)

`pages → widgets/layout → components`; `pages/widgets → services → backend`; усі можуть брати
`types`, `domain`, `hooks`, `state`. `components/` не знає домену. `domain/`/`types/` не імпортують
`leptos`/`sea_orm` (у `types/` допустимий `serde`).

| Елемент | Що це | Хто використовує |
|---|---|---|
| `app.rs` | `shell()` + `App`: провайдери контекстів, `<Router>` | `server` (SSR/fallback) |
| `routes.rs` | таблиця маршрутів → `pages::*` | `app.rs` |
| `backend/` | лише `ssr`: SQL (`repo/`), права (`policy`), доступ до БД (`db`) | `services/`, `pages/*/server.rs` |
| `domain/` | чиста логіка без UI/БД (WASM) | будь-хто |
| `hooks/` | реактивні хелпери (`use_actor`) | будь-хто |
| `layout/` | каркас застосунку (шапка) | `app.rs` |
| `pages/` | сторінки-маршрути | `routes.rs` |
| `services/` | `#[server]`-функції для ≥ 2 споживачів | `layout/`, `pages/` |
| `types/` | DTO клієнт↔сервер | будь-хто |
| `widgets/` | доменні компоненти для ≥ 2 сторінок (`ActorNotice`) | `pages/`, `layout/` |

`components/`/`state/` не існують: не створювати наперед (§1).
