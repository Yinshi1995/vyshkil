# app/src/pages/home — головна сторінка (`/`)

- Можна: `services`, `widgets`, `hooks`, `types`, `domain`, `backend` (лише з `server.rs`, під `ssr`).
  Не можна: інші сторінки.
- `server.rs`/`components/` оголошені без `pub` (`mod server; mod components;`) — приватні для
  цієї сторінки. Компонент/server fn стає потрібним ще одній сторінці → переносити в
  `widgets/`/`services/` (07 §2.1), не копіювати.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `HomePage` — лічильники + `<OrgSearch/>` + `<SubordinationTree/>` | `pages/CLAUDE.md` → `routes.rs` |
| `server.rs` | `get_subordination_tree` — потрібен лише цій сторінці (`search_orgs` переїхав у `services/orgs.rs`, потрібен ще й `training_form`) | `components/` |
| `components/` | `OrgSearch`, `SubordinationTree` (своя карта) | `mod.rs` |
