# app/src — серверна бібліотека (після видалення Leptos, 2026-11-06)

Крейт `app` — це лише серверна логіка; інтерфейс — `web/` (React), HTTP — `server/` (Axum).
Рішення: `.claude/decisions/remove-leptos-fallback.md`.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `backend/` | SQL/SeaORM (`repo/`), права (`policy.rs`), імпорт файлів (`import/`), генератори документів (`documents/`), `db.rs` (транзакція з `SET LOCAL app.actor`) | `server/src/api.rs` та інші |
| `domain/` | чиста логіка без БД (нормалізація, дати, лічильники) | `backend/`, `server/` |
| `types/` | DTO (`actor`, `auth`, `submission`…) | `backend/`, `server/` |
| `services/` (лише `auth.rs`) | прапорець `PASSWORD_LOGIN`, ліміт спроб входу | `server/src/api.rs`, `main.rs`, `oidc.rs` |

Правила: SQL — лише в `backend/repo/`; перевірка прав — лише в `backend/policy.rs`; `domain/` і `types/` без `sea_orm`.
