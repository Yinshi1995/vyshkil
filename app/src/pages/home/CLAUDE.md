# app/src/pages/home — головна сторінка (`/`)

- Можна: `services`, `widgets`, `hooks`, `types`, `domain`, `backend` (лише з `server.rs`, під `ssr`).
  Не можна: інші сторінки.
- `server.rs`/`components/` оголошені без `pub` (`mod server; mod components;`) — приватні для
  цієї сторінки. Компонент/server fn стає потрібним ще одній сторінці → переносити в
  `widgets/`/`services/` (07 §2.1), не копіювати.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `HomePage` — дашборд: статистика + швидкі дії + останні подання + `<OrgSearch/>` + `<SubordinationTree/>` | `pages/CLAUDE.md` → `routes.rs` |
| `server.rs` | `get_subordination_tree`, `get_dashboard_stats`, `get_recent_submissions` — потрібні лише цій сторінці | `mod.rs`, `components/` |
| `components/` | `OrgSearch`, `SubordinationTree` (своя карта) | `mod.rs` |
