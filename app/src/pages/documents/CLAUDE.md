# app/src/pages/documents — генерація документів (`/documents`, 05, Етап 7)

- Можна: `services`, `widgets`, `hooks`, `types`, `domain`, `backend` (лише з `server.rs`, під `ssr`).
  Не можна: інші сторінки.
- `server.rs` оголошений без `pub` (`mod server;`) — приватний для цієї сторінки.
- **Перший вертикальний зріз — лише D1** (один денний аркуш): вибір органу (`widgets::group_grid::
  OrgAutocomplete`, звужено до видимого акторові піддерева — той самий компонент, що й Grid) + дата
  (`components::DatePicker`) → `#[server]` повертає готові байти xlsx напряму (не читання з диска
  клієнтом) → `components::download_bytes` ініціює браузерне скачування через `Blob`+`<a download>`.
  Копія лишається на диску + рядок `generated_document` (05 §вступ) для майбутньої історії
  генерацій — ще не побудована сторінка перегляду цієї історії.
- Право — `policy::can_view_org` (звіт читає дані, не пише) — НЕ `can_edit_org`.
- Розмітка — атоми `style`-крейту (skill `styling`), не BEM-CSS: перша версія помилково завела
  невизначені `.documents__*`-класи (нова сторінка мала одразу йти атомами, `pages/CLAUDE.md`).
  `OrgAutocomplete` тут — поза `Grid`, тому обгорнутий у `<div class=cx!("w-full bg-raised bd r1")>`:
  сам компонент рендерить голий `.cell__input` (`background: transparent`, майже без рамки) — у
  `Grid` видимість дає хром сітки (межі клітинок), поза нею інпут без обгортки виглядає порожнім
  місцем, не полем вводу (саме так і сталось при першій версії сторінки).
- D2 (xlsx, накопичувальна)/D3 (docx)/D4 (pptx-за-шаблоном, наразі відкладено — еталон
  `docs/templates/presentation-style.pptx` фізично відсутній у репозиторії) — окремі кроки після.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `DocumentsPage`, `DocumentsBody` — вибір органу/дати, виклик генерації, тригер скачування | `pages/CLAUDE.md` → `routes.rs` |
| `server.rs` | `generate_d1` — `policy::can_view_org` → `repo::documents::daily_training_rollup` → `backend::documents::d1::build_day_sheet` → байти + запис на диск/`generated_document` | `mod.rs` |
