# app/src/pages/org_detail — картка частини (`/org/:id`)

- Можна: `services`, `widgets`, `hooks`, `types`, `domain`, `backend` (лише з `server.rs`, під `ssr`).
  Не можна: інші сторінки.
- `server.rs` оголошений без `pub` (`mod server;`) — приватний для цієї сторінки.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `OrgDetailPage` — дані організації, підлеглі, групи, подання, користувачі (admin), історія | `pages/CLAUDE.md` → `routes.rs` |
| `server.rs` | `get_org_detail`, `get_org_children`, `get_org_groups`, `get_org_submissions`, `get_org_users` | `mod.rs` |
