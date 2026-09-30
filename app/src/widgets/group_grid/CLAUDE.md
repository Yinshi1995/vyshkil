# app/src/widgets/group_grid — сітка рядків-груп з клавіатурою й автокомплітом

Переїхало з `pages/training_form/components/` (Етап 5): та сама сітка тепер потрібна й
`pages/import` (03 §6: "Таблиця як у формі введення (02)") — 07 §2.1, використовується ≥ 2
сторінками.

- Можна: `services`, `hooks`, `types`, `domain`. Не можна: `pages`, `backend`.
- `Callback<T>` тут — з `leptos::prelude`: для одного нетюпльованого аргументу (`Callback<String>`,
  `Callback<web_sys::KeyboardEvent>`) конструюй явно через `Callback::new(...)`, не передавай сирий
  closure напряму (`.into()` для одноаргументної нетюпльованої форми в цій версії Leptos не
  реалізовано) — для 2+ аргументів (`Callback<(P1, P2)>`) навпаки: closure з окремими параметрами
  (`move |a, b| ...`), не `move |(a, b)| ...`.
- Свідомі спрощення (задокументовано в коді): віртуалізація — прогресивне дорендерювання, не
  ковзне вікно; вставка з Excel не підключена.
- "Розгорнути рядок" (feedback користувача — "як Notion") — `.grid__expand` у кожному `Row`,
  спільний `components::Drawer` на всю `Grid` (не по одному на рядок), тримає посилання на
  `RwSignal<GroupFormRow>` обраного рядка. `tabindex="-1"` навмисно — поза Tab-послідовністю
  (02 §1 колонка 8, "Розподіл за підрозділами" — тепер тут, `RowEditor::CompositionEditor`,
  перший реальний спосіб РУЧНОГО введення composition, раніше лише з імпорту).
- `TrainingKindCell`/`SiteCell` — `components::Combobox variant=InCell` (не голий `<select>`):
  стрілки/Enter, поки список закритий, форвардяться в `on_cell_keydown` (сітка лишається власником
  навігації МІЖ клітинками), відкриття — `Alt+↓` (той самий шорткат, що вже в шпаргалці).
- **Відома, поза обсягом межа**: `ArrowDown` на ОСТАННЬОМУ рядку створює новий рядок і намагається
  сфокусувати його СИНХРОННО, до монтування — фокус лишається на місці. Будь-яка клітинка.
- **Розділ Б** — 10 живих колонок (було 14, секундарні поля лише в `RowEditor`); `Grid` має новий
  проп `as_of: Signal<NaiveDate>`; `.grid` — ОДИН sticky-скрол-контейнер. Деталі — `docs/spec/
  components/grid.md`, `.claude/decisions/grid-column-strategy.md`.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `grid.rs` | `Grid`, `EditableRow`/`wrap_rows`/`snapshot_rows` (рядок = власний сигнал, щоб фокус не губився при перерендері); `NumberCell` (права юстиція, placeholder "—" замість "0") | `pages/training_form/mod.rs`, `pages/import/mod.rs` |
| `columns.rs` | `use_columns`/`ColumnsState`/`DATA_COLUMNS`/`start_resize` — ширини/видимість колонок, `localStorage` (ручне CSV-кодування — `serde_json` лише під фічею `ssr`, недоступний тут) | `grid.rs` |
| `date_range_cell.rs` | `DateRangeCell` — "З"/"По" пов'язані як діапазон, парсинг через `domain::dates` (гнучкий, не 8-цифровий mask `DatePicker`), спільний календар-поповер (portal) | `grid.rs` |
| `row_menu.rs` | `RowMenu` — меню рядка (дублювати/видалити), portal, НЕ `Combobox` (дія, не значення) | `grid.rs` |
| `autocomplete.rs` | `OrgAutocomplete` (реекспортовано з `mod.rs`), `VosPositionCourseAutocomplete` (02 §3) — тонкі обгортки над `components::Combobox mode=Input` (розділ Б, `docs/spec/components/grid.md` §2): тримають `Resource`+`Vec<ComboboxItem>`, компонент лишається domain-agnostic | `grid.rs`, `row_editor.rs`, `pages::documents` |
| `row_editor.rs` | `RowEditor` — форма всіх полів рядка в `Drawer` ("як Notion"), `CompositionEditor` | `grid.rs` |
