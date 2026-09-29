# app/src/pages/training_form — сітка введення груп (`/training-form`, 02)

- Можна: `services`, `widgets`, `hooks`, `types`, `domain`, `backend` (лише з `server.rs`, під `ssr`).
  Не можна: інші сторінки.
- `server.rs`/`components/` оголошені без `pub` (`mod server; mod components;`) — приватні для
  цієї сторінки.
- Права: `viewer` не зберігає (ні чернетку, ні коміт); `org_editor` — лише власна організація
  (`policy::can_edit_org` перевіряється в `commit_grid` на сервері, ПЕРЕД записом).
- Свідомі спрощення відносно повної 02 (задокументовано в коді, не приховано): віртуалізація —
  прогресивне дорендерювання, не ковзне вікно (`components/grid.rs`); "Розподіл за підрозділами"
  (колонка 8) не в Tab-послідовності цієї версії; вставка з Excel іще не підключена (Ctrl+V) —
  чекає на розпізнавання/валідацію з Етапу 5 (03), про яке явно каже сама специфікація (02 §7).
- e2e (Playwright, критерій готовності Етапу 4) — `e2e/tests/training-form.spec.ts`, окремий
  Node-проєкт (не Cargo workspace). Перед запуском: `docker start taktoblik-db`,
  `cargo leptos build`, підняти `server.exe`, `DELETE FROM group_event; DELETE FROM training_group;
  DELETE FROM submission;` (тест припускає порожню `submission`), тоді `cd e2e && npx playwright
  test` (перший раз — `npm install && npx playwright install chromium`).
- Дві реальні пастки, спіймані саме через e2e (не через `cargo test`, бо це UI-реактивність):
  1) `wrap_rows` мав приймати `&mut u32`, а `StoredValue::get_value()` повертає копію — інкремент
     ніколи не писався назад, тож відновлення чернетки видавало той самий `id`, що й уже змонтований
     рядок; `<For>` бачив однаковий ключ і НЕ перемонтовував рядок — DOM показував старий сигнал,
     доки автозбереження/коміт читали новий (порожній). Тепер `wrap_rows` бере `StoredValue<u32>`
     напряму.
  2) Клітинковий обробник `Enter` не перевіряв `ev.ctrl_key()` — `Ctrl+Enter` (глобальне "зберегти
     все") одночасно спрацьовував і як "перейти далі" в активній клітинці, створюючи зайвий
     порожній рядок ПІД ЧАС коміту. Усі "Enter"-гілки (сітка й обидва автокомпліти) тепер явно
     виключають `ev.ctrl_key()`.
  3) `GroupFormRow.composition: Vec<CompositionRow>` без `#[serde(default)]` — form-урленкодед
     аргументи server fn (Leptos) не передають ключ для порожнього `Vec` узагалі; без `default`
     десеріалізація падала "missing field" щоразу, як підрозділи не заповнені (типовий випадок).

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `TrainingFormPage` — стан сітки, автозбереження, undo/redo, глобальні гарячі клавіші, шпаргалка, командна палітра | `pages/CLAUDE.md` → `routes.rs` |
| `server.rs` | `get_draft`/`save_draft`/`commit_grid`, `search_vos_position_course`, `get_training_sites` | `mod.rs`, `components/` |
| `components/` | `Grid` (своя карта) | `mod.rs` |
