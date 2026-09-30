# app/src/pages/discrepancies — екран розбіжностей (`/discrepancies`, 04, Етап 8 зріз 1)

- Можна: `services`, `widgets`, `hooks`, `types`, `domain`, `backend` (лише з `server.rs`, під `ssr`).
  Не можна: інші сторінки.
- `server.rs` оголошений без `pub` (`mod server;`) — приватний для цієї сторінки.
- **Лише перегляд** (`.claude/decisions/etap8-horizontal-reconciliation-first-slice.md`) —
  відкриття/автозакриття розбіжностей відбувається при фіксації сітки
  (`backend::repo::reconciliation::refresh_horizontal`, викликається з `repo::groups::
  commit_group_rows`), не тут. Workflow "взяти в роботу"/"закрити вручну" — наступний зріз.
- Права: `policy::visible_org_ids` звужує список (той самий принцип, що `pages/home`) — `repo`
  сам прав не перевіряє.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `DiscrepanciesPage`, `DiscrepancyTable` — список + фільтр статусу | `pages/CLAUDE.md` → `routes.rs` |
| `server.rs` | `get_discrepancies` — потрібен лише цій сторінці | `mod.rs` |
