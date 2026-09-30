# app/src/pages/documents — генерація документів (`/documents`, 05, Етап 7)

- Можна: `services`, `widgets`, `hooks`, `types`, `domain`, `backend` (лише з `server.rs`, під `ssr`).
  Не можна: інші сторінки.
- `server.rs` оголошений без `pub` (`mod server;`) — приватний для цієї сторінки.
- **D1** (один денний аркуш): вибір органу (`widgets::group_grid::OrgAutocomplete`, звужено до
  видимого акторові піддерева) + дата (`components::DatePicker`) → `#[server]` повертає готові
  байти xlsx напряму → `components::download_bytes` (Blob+`<a download>`).
- **D2** ("Контролька", один тиждень): лише дата (будь-який день тижня, сервер сам рахує пн..нд) —
  без вибору органу, D2 завжди охоплює всі корпуси одразу (`repo::documents::top_level_orgs`).
  Право перевіряється на КОЖЕН з корпусів окремо (актор може не бачити всіх).
  `org_id` у `generated_document` для D2 — корінь ієрархії (`root_org_id`), не "власник"-орган.
  Обидва — той самий `#[server]`→байти→`download_bytes` шлях, копія на диск + `generated_document`
  (05 §вступ) для майбутньої історії генерацій — ще не побудована сторінка перегляду цієї історії.
- Право — `policy::can_view_org` (звіт читає дані, не пише) — НЕ `can_edit_org`.
- Розмітка — атоми `style`-крейту (skill `styling`), не BEM-CSS: перша версія помилково завела
  невизначені `.documents__*`-класи (нова сторінка мала одразу йти атомами, `pages/CLAUDE.md`).
  `OrgAutocomplete` тут — поза `Grid`, тому обгорнутий у `<div class=cx!("w-full bg-raised bd r1")>`:
  сам компонент рендерить голий `.cell__input` (`background: transparent`, майже без рамки) — у
  `Grid` видимість дає хром сітки (межі клітинок), поза нею інпут без обгортки виглядає порожнім
  місцем, не полем вводу (саме так і сталось при першій версії сторінки).
- D3 (docx)/D4 (pptx-за-шаблоном) — окремі кроки після. `docs/templates/presentation-style.pptx`
  для D4 — файл ІСНУЄ в репо з базового коміту (`b07161a`), просто був недоступний агенту через
  `permissions.deny` (`Read(*.pptx)`/`Read(docs/templates/**)`, прибрано 2026-09-29).

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `DocumentsPage`/`DocumentsBody` (`D1Block`+`D2Block`) — вибір органу/дати, виклик генерації, тригер скачування | `pages/CLAUDE.md` → `routes.rs` |
| `server.rs` | `generate_d1`/`generate_d2` — `policy::can_view_org` → `repo::documents::daily_training_rollup`(+`top_level_orgs`/`root_org_id` для D2) → `backend::documents::d1`/`d2` → байти + запис на диск/`generated_document` | `mod.rs` |
