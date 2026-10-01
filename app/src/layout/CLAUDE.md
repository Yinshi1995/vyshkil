# app/src/layout — каркас застосунку (шапка, PageHeader/Toolbar/PageContent)

- Можна: `services`, `hooks`, `types`, `domain`. Не можна: `pages`, `backend` (тільки через `services`).
- `actor_switcher.rs` приватний для цієї теки (використовує лише `header.rs`) — якщо колись
  знадобиться деінде, переносити в `widgets/` (07 §2.1), не копіювати.
- **Етап 7.5** (`docs/spec/components/layout.md`) — `main` (app.rs) БЕЗ власного `max-width`;
  кожна сторінка сама оголошує ширину через `PageContent`. Усі 11 сторінок мігровано.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `header.rs` | `Header` — лого + `ActorSwitcher` | `app.rs` |
| `actor_switcher.rs` | `ActorSwitcher` — вибір org+роль (dev-заміна автентифікації, 01 §6) | лише `header.rs` |
| `page_header.rs` | `PageHeader`/`Breadcrumb` — компактний заголовок+хлібні крихти+підзаголовок+дії | усі `pages::*` |
| `toolbar.rs` | `Toolbar` — `display:flex;space-between`, викликач сам дає 2 групи-дітей | `pages::training_form`, `pages::import` |
| `content.rs` | `PageContent`/`ContentWidth` — тип ширини контенту: `Data` (сітки), `Detail` (головна/картка частини/довідники), `Reading` (ВОС за ОВТ) | усі `pages::*` |
