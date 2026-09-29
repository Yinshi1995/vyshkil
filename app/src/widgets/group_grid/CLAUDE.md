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
  ковзне вікно; "Розподіл за підрозділами" поза Tab-послідовністю; вставка з Excel не підключена.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `grid.rs` | `Grid`, `EditableRow`/`wrap_rows`/`snapshot_rows` (рядок = власний сигнал, щоб фокус не губився при перерендері) | `pages/training_form/mod.rs`, `pages/import/mod.rs` |
| `autocomplete.rs` | `OrgAutocomplete`, `VosPositionCourseAutocomplete` (02 §3) | `grid.rs` |
