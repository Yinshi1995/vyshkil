# app/src/pages/org_detail — картка частини (`/org/:id`)

- Можна: `services`, `types`, `domain`, `backend` (лише з `server.rs`, під `ssr`).
  Не можна: інші сторінки.
- `server.rs` оголошений без `pub` (`mod server;`) — приватний для цієї сторінки.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `OrgDetailPage` — поточні дані + історія назв/статусів | `pages/CLAUDE.md` → `routes.rs` |
| `server.rs` | `get_org_detail` — потрібен лише цій сторінці | `mod.rs` |
