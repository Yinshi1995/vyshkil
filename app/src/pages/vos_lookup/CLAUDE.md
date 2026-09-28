# app/src/pages/vos_lookup — підказка "ОВТ/сленг → ВОС" (`/vos-lookup`)

- Можна: `services`, `widgets`, `hooks`, `types`, `domain`, `backend` (лише з `server.rs`, під `ssr`).
  Не можна: інші сторінки.
- `server.rs` оголошений без `pub` (`mod server;`) — приватний для цієї сторінки. Не org-scoped
  дані (довідник, не `policy`) — на відміну від `pages/home`/`pages/org_detail`.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `VosLookupPage` — поле пошуку + список підказок | `pages/CLAUDE.md` → `routes.rs` |
| `server.rs` | `get_equipment_vos_hint` — потрібен лише цій сторінці | `mod.rs` |
