# app/src/pages/import — превʼю імпорту (`/import`, 03/Етап 5 — КВід/ІВС; + Етап 6 архів)

- Можна: `services`, `widgets`, `hooks`, `types`, `domain`, `backend` (лише з `server.rs`, під `ssr`).
  Не можна: інші сторінки.
- `server.rs` оголошений без `pub` (`mod server;`) — приватний для цієї сторінки.
- Права: `viewer` не імпортує; `org_editor` — комітиться лише за власну організацію (той самий
  `policy::can_edit_org` у `commit_grid_impl`, `services/submission_grid.rs`, що й у формі).
- **Фах/БпС/Терміни переїхали на `/training-form`** (feedback користувача — об'єднання ручного
  вводу й імпорту, [[unified-training-form-source-type]]): вони чисті виробники `GroupFormRow` без
  побічного виводу й без окремого `source_type`, тож об'єднання з формою було безболісним. Тут
  лишились лише три типи, яким об'єднання НЕ підходить — перемикач `components::Select` у
  `ImportBody` (`FileKind::key()`/`from_key()` — рядковий міст).
- **Kvid** → зовсім інша форма даних (`StaffingRow`, 01 §4) → окрема проста `StaffingTable`, без
  undo/автозбереження → `commit_staffing` (пише `staffing_snapshot`/`_metric` напряму, не через
  `submission_grid`). **Ivs** — ОБИДВІ форми з одного файлу: `staffing`→ `InstructorStaffingTable`,
  `groups`→ той самий `Grid` (стажування/курси — "рядок групи" з `training_kind='internship'`/
  `'special'+course_id`), `Ctrl+Enter` комітить обидва окремими викликами (не атомарно) — саме
  тому лишився тут, а не переїхав з рештою: розділити на дві сторінки означало б два UI для
  одного файлу.
- **VchArchive (Етап 6) — той самий `Grid`, ІНШИЙ `submission.source_type`**: `FileKind::
  is_archive()` → `source_type='archive_seed'` (окреме тріо `get_archive_draft`/`save_archive_draft`/
  `commit_archive_grid` у `server.rs`, не runtime-параметр — джерело типізоване константою на
  клієнті). Без автозбереження й без `submission_id`-неперервності (коміт завжди з `None` —
  одноразовий перенос, чернетка-в-часі не потрібна). Статус превʼю рахує рядки з нерозпізнаною
  частиною — "звіт переносу" роадмапу Етапу 6; сам коміт лишається все-або-нічого.
- Файл читається в байти на клієнті (`components::read_file_bytes`, `File::array_buffer()`, без
  multipart) і йде як `Vec<u8>` у `#[server]`-аргументі — спільна з `training_form` (обидві
  сторінки читають файл однаково).
- Дублює частину каркасу `training_form/mod.rs` (автозбереження, undo/redo, гарячі клавіші) —
  свідомо, поки не з'явиться третій споживач `widgets::group_grid` з тим самим патерном сторінки
  (07 §1: правило двох; VchArchive — той самий споживач, не новий).

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `ImportPage`, `ImportBody`, `StaffingTable`/`StaffingTableRow` (Kvid), `InstructorStaffingTable`/`InstructorStaffingTableRow` (Ivs) | `pages/CLAUDE.md` → `routes.rs` |
| `server.rs` | `parse_kvid_file`/`parse_ivs_file`/`parse_vch_archive_file`, `commit_staffing`/`commit_instructor_staffing`, `get_draft`/`save_draft`/`commit_grid` (`table`, Ivs-групи), `get_archive_draft`/`save_archive_draft`/`commit_archive_grid` (`archive_seed`) — тонкі обгортки над `services::submission_grid` | `mod.rs` |
