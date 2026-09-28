# app/src/pages/training_form/components — сітка і поля з автокомплітом

- Можна: `pages::training_form::server` (приватні модулі однієї сторінки бачать одне одного),
  `services`, `hooks`, `types`, `domain`. Не можна: інші сторінки.
- Використовує ще й інша сторінка → переносити у `widgets/` (07 §2.1), не копіювати.
- `Callback<T>` тут — з `leptos::prelude`: для одного нетюпльованого аргументу (`Callback<String>`,
  `Callback<web_sys::KeyboardEvent>`) конструюй явно через `Callback::new(...)`, не передавай сирий
  closure напряму (`.into()` для одноаргументної нетюпльованої форми в цій версії Leptos не
  реалізовано) — для 2+ аргументів (`Callback<(P1, P2)>`) навпаки: closure з окремими параметрами
  (`move |a, b| ...`), не `move |(a, b)| ...`.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `grid.rs` | `Grid`, `EditableRow`/`wrap_rows`/`snapshot_rows` (рядок = власний сигнал, щоб фокус не губився при перерендері) | `pages/training_form/mod.rs` |
| `autocomplete.rs` | `OrgAutocomplete`, `VosPositionCourseAutocomplete` (02 §3) | `grid.rs` |
