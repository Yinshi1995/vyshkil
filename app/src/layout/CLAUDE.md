# app/src/layout — каркас застосунку (шапка, майбутня навігація)

- Можна: `services`, `types`, `domain`. Не можна: `pages`, `backend` (тільки через `services`).
- `actor_switcher.rs` приватний для цієї теки (використовує лише `header.rs`) — якщо колись
  знадобиться деінде, переносити в `widgets/` (07 §2.1), не копіювати.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `header.rs` | `Header` — лого + `ActorSwitcher` | `app.rs` |
| `actor_switcher.rs` | `ActorSwitcher` — вибір org+роль (dev-заміна автентифікації, 01 §6) | лише `header.rs` |
