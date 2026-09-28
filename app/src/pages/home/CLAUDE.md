# app/src/pages/home — головна сторінка (`/`)

- Можна: `services`, `types`, `domain`, `backend` (лише з `server.rs`, під `ssr`).
  Не можна: інші сторінки.
- `server.rs`/`components/` оголошені без `pub` (`mod server; mod components;`) — приватні для
  цієї сторінки. Компонент/server fn стає потрібним ще одній сторінці → переносити в
  `widgets/`/`services/` (07 §2.1), не копіювати.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `HomePage` — лічильники + `<OrgSearch/>` + `<SubordinationTree/>` | `pages/CLAUDE.md` → `routes.rs` |
| `server.rs` | `search_orgs`, `get_subordination_tree` — потрібні лише цій сторінці | `components/` |
| `components/` | `OrgSearch`, `SubordinationTree` (своя карта) | `mod.rs` |
