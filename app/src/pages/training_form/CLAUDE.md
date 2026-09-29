# app/src/pages/training_form — об'єднана сітка ручного вводу й імпорту (`/training-form`, 02)

- Можна: `services`, `widgets`, `hooks`, `types`, `domain`, `backend` (лише з `server.rs`, під `ssr`).
  Не можна: інші сторінки.
- `server.rs` оголошений без `pub` (`mod server;`) — приватний для цієї сторінки. Сама сітка
  (`Grid`) переїхала в `widgets/group_grid/` (Етап 5: знадобилась і `pages/import`) — див. її карту.
- Права: `viewer` не зберігає (ні чернетку, ні коміт); `org_editor` — лише власна організація
  (`policy::can_edit_org` перевіряється в `commit_grid` на сервері, ПЕРЕД записом).
- `server.rs` тепер лише тонкі `#[server]`-обгортки над `services::submission_grid::*_impl`
  (`source_type="form"`) — спільна логіка з `pages/import` (Етап 5).
- **Об'єднання з імпортом (feedback користувача, [[unified-training-form-source-type]])**: Фах/БпС/
  Терміни (`FileKind` тут, лише 3 варіанти) переїхали з `pages::import` разом зі своїми
  `parse_*_file` — та сама `Grid`, той самий `source_type='form'`, немає більше окремої чернетки
  "превʼю імпорту". `FileDropzone` у шапці — якщо в сітці вже є непорожні рядки, розібрані рядки
  ДОДАЮТЬСЯ в кінець (`on_file_selected` у `mod.rs`), інакше замінюють єдиний порожній
  рядок-заглушку. КВід/ІВС/Архів ВЧ лишились на `/import` — інша форма даних або окремий
  `source_type`, об'єднання їм не підходить (`pages/import/CLAUDE.md`).
- e2e (Playwright, критерій готовності Етапу 4) — `e2e/tests/training-form.spec.ts`, окремий
  Node-проєкт (не Cargo workspace). Перед запуском: `docker start taktoblik-db`,
  `cargo leptos build`, підняти `server.exe`, `DELETE FROM group_event; DELETE FROM training_group;
  DELETE FROM submission;` (тест припускає порожню `submission`), тоді `cd e2e && npx playwright
  test` (перший раз — `npm install && npx playwright install chromium`).
- Три реальні пастки, спіймані саме через e2e (не через `cargo test`, бо це UI-реактивність) —
  деталі тепер у `widgets/group_grid/CLAUDE.md` (де й живе код, що їх виправляє): зламаний
  лічильник id у `wrap_rows`, `Ctrl+Enter` без перевірки `ev.ctrl_key()` в клітинковому `Enter`,
  `Vec<CompositionRow>` без `#[serde(default)]`.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `TrainingFormPage` — стан сітки, автозбереження, undo/redo, глобальні гарячі клавіші, шпаргалка, командна палітра, `FileKind`/`on_file_selected` (Фах/БпС/Терміни) | `pages/CLAUDE.md` → `routes.rs` |
| `server.rs` | `get_draft`/`save_draft`/`commit_grid` (тонкі обгортки над `services::submission_grid`), `parse_fah_file`/`parse_bps_file`/`parse_terminy_file` | `mod.rs` |
