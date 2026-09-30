# app/src/widgets/group_grid — сітка рядків-груп з клавіатурою й автокомплітом

Переїхало з `pages/training_form/components/` (Етап 5): та сама сітка тепер потрібна й
`pages/import` (03 §6: "Таблиця як у формі введення (02)") — 07 §2.1, використовується ≥ 2
сторінками.

- Можна: `services`, `hooks`, `types`, `domain`. Не можна: `pages`, `backend`.
- `Callback<T>` — з `leptos::prelude`: 1 аргумент → `Callback::new(move |x| ...)`, не сирий closure;
  2+ (`Callback<(P1,P2)>`) → closure з окремими параметрами (`move |a, b| ...`).
- Свідомі спрощення: віртуалізація — прогресивне дорендерювання, не ковзне вікно; Excel-вставка
  не підключена.
- "Розгорнути рядок" ("як Notion") — один спільний `components::Drawer` на всю `Grid`, тримає
  `RwSignal<GroupFormRow>` обраного рядка. `TrainingKindCell`/`SiteCell` — `Combobox variant=
  InCell`, стрілки/Enter форвардяться в `on_cell_keydown` (сітка — власник навігації між
  клітинками), відкриття — `Alt+↓`.
- **Відома, поза обсягом межа**: `ArrowDown` на ОСТАННЬОМУ рядку створює новий рядок і намагається
  сфокусувати його СИНХРОННО, до монтування — фокус лишається на місці. Будь-яка клітинка.
- **Грід-переробка після відхилення (2026-09-30)** — контракт клавіатури тепер `docs/spec/
  components/grid-interaction.md` (не цей файл), обґрунтування — `.claude/decisions/
  sender-org-once-per-submission.md`. Стисло: 9 живих колонок (Частина прибрана в тулбар
  сторінки, дефект 1 — `Grid` бере `columns: ColumnsState` ЗЗОВНІ, ділиться з `<ColumnsToggle>`
  у тулбарі, дефект 8); `components::combobox` in-cell тепер відкриває на фокусі одразу з
  "недавні → весь довідник" (дефект 3, `autocomplete.rs`'s `localStorage`-recent), Tab-через-
  клітинку не мутує (`touched`-прапорець, дефект 4); `date_range_cell.rs` — жива маска через
  `domain::dates::format_date_mask` (дефект 5); порожня клітинка мовчить у спокої (дефект 10);
  "План→Прибуло/Навчаються"-підказка (§4 контракту, `Row`'s `*_suggested` сигнали).

| Елемент | Що це | Хто використовує |
|---|---|---|
| `grid.rs` | `Grid` (бере `columns: ColumnsState` ЗЗОВНІ), `EditableRow`/`wrap_rows`/`snapshot_rows`; `NumberCell` (права юстиція, placeholder "—" замість "0", `suggested`-стан) — 9 живих колонок (Частина прибрана) | `pages/training_form/mod.rs`, `pages/import/mod.rs` |
| `columns.rs` | `use_columns`/`ColumnsState`/`DATA_COLUMNS`/`start_resize`/`ColumnsToggle` (кнопка+панель "Колонки" — тепер у тулбарі сторінки) — ширини/видимість колонок, `localStorage` (ручне CSV-кодування — `serde_json` лише під фічею `ssr`, недоступний тут) | `grid.rs`, `pages/training_form/mod.rs`, `pages/import/mod.rs` |
| `date_range_cell.rs` | `DateRangeCell` — "З"/"По" пов'язані як діапазон, парсинг через `domain::dates` (гнучкий, не 8-цифровий mask `DatePicker`), спільний календар-поповер (portal) | `grid.rs` |
| `row_menu.rs` | `RowMenu` — меню рядка (дублювати/видалити), portal, НЕ `Combobox` (дія, не значення) | `grid.rs` |
| `autocomplete.rs` | `OrgAutocomplete` (реекспортовано з `mod.rs`), `VosPositionCourseAutocomplete` (02 §3) — тонкі обгортки над `components::Combobox mode=Input` (розділ Б, `docs/spec/components/grid.md` §2): тримають `Resource`+`Vec<ComboboxItem>`, компонент лишається domain-agnostic | `grid.rs`, `row_editor.rs`, `pages::documents` |
| `row_editor.rs` | `RowEditor` — форма всіх полів рядка в `Drawer` ("як Notion"), `CompositionEditor` | `grid.rs` |
