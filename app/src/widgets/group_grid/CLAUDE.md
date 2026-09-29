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
- `TrainingKindCell`/`SiteCell` — `components::Select` з `id`/`on_keydown` (не голий `<select>`):
  стрілки/Enter, поки список закритий, форвардяться в `on_cell_keydown` (сітка лишається власником
  навігації МІЖ клітинками), відкриття — `Alt+↓` (той самий шорткат, що вже в шпаргалці).
- **Відома, поза обсягом межа** (не моя регресія — той самий баг був і до `Select`): `ArrowDown` на
  ОСТАННЬОМУ рядку створює новий рядок і намагається сфокусувати його СИНХРОННО — новий рядок ще
  не встиг змонтуватись у DOM, фокус лишається на місці (не губиться на `BODY`, `focus_cell` просто
  не знаходить елемент). Відтворюється на будь-якій клітинці, не лише `Select`.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `grid.rs` | `Grid`, `EditableRow`/`wrap_rows`/`snapshot_rows` (рядок = власний сигнал, щоб фокус не губився при перерендері) | `pages/training_form/mod.rs`, `pages/import/mod.rs` |
| `autocomplete.rs` | `OrgAutocomplete` (реекспортовано з `mod.rs` — Етап 7 забрав другого споживача поза `Grid`), `VosPositionCourseAutocomplete` (02 §3) | `grid.rs`, `pages::documents` |
| `row_editor.rs` | `RowEditor` — форма всіх полів рядка в `Drawer` ("як Notion"), `CompositionEditor` | `grid.rs` |
