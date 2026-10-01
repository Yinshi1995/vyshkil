# app/src/pages/documents — генерація документів (`/documents`, 05, Етап 7)

- Можна: `services`, `widgets`, `hooks`, `types`, `domain`, `backend` (лише з `server.rs`, під `ssr`).
  Не можна: інші сторінки.
- `server.rs` оголошений без `pub` (`mod server;`) — приватний для цієї сторінки.
- **D1** (повний тижневий файл): вибір органу + будь-який день тижня →
  сервер рахує пн–нд, 7 денних аркушів + приховані "початок"/"кінець" (маркери 3D-діапазону) +
  "тижневий" `SUM(початок:кінець!C5)` → `components::download_bytes` (Blob+`<a download>`).
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
- **D3** (docx "Говорілка"): дата → rollup усіх корпусів (сьогодні + вчора для дельт) →
  `d3::render_paragraphs` (шаблон `{{placeholder}}`, `@corps` для ітерації) → `d3::build_docx`
  (zip/XML). Шаблон за замовчуванням вбудований; адмін може підмінити файлом через env
  `DOCUMENTS_D3_TEMPLATE`. MIME: `application/vnd.openxmlformats-officedocument.wordprocessingml.document`.
- **D4** (pptx "Підготовка"): дата → ті ж rollup дані → `d4::build_pptx` (zip/XML від scratch,
  KPI-плитки + таблиця по корпусах). Стиль за 05 §D4 spec (фон `#0E0C08`, акцент `#F39200`).
  Шаблонний підхід із іменованими фігурами потребує `source_files/Підготовка_26.09.2026.pptx`
  (docs/QUESTIONS.md). `docs/templates/presentation-style.pptx` — лише стильовий еталон.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `DocumentsPage`/`DocumentsBody` (`D1Block`+`D2Block`+`D3Block`+`D4Block`) — вибір органу/дати, виклик генерації, тригер скачування | `pages/CLAUDE.md` → `routes.rs` |
| `server.rs` | `generate_d1`/`generate_d2`/`generate_d3`/`generate_d4` — `policy::can_view_org` → `repo::documents::daily_training_rollup`(+`top_level_orgs`/`root_org_id`) → `backend::documents::d1`-`d4` → байти + запис на диск/`generated_document` | `mod.rs` |
