# app/src/pages/import — превʼю імпорту (`/import`, 03/Етап 5 — усі 5 типів файлу; + Етап 6 архів)

- Можна: `services`, `widgets`, `hooks`, `types`, `domain`, `backend` (лише з `server.rs`, під `ssr`).
  Не можна: інші сторінки.
- `server.rs` оголошений без `pub` (`mod server;`) — приватний для цієї сторінки.
- Права: `viewer` не імпортує; `org_editor` — комітиться лише за власну організацію (той самий
  `policy::can_edit_org` у `commit_grid_impl`, `services/submission_grid.rs`, що й у формі).
- Шість типів файлу (Фах, БпС, КВід, ІВС, Терміни — Етап 5; Архів ВЧ — Етап 6) — перемикач
  `<select>` у `ImportBody`, `FileKind` вирішує, який `parse_*_file` викликати.
- **Fah/Bps/Terminy/VchArchive → `GroupFormRow` → та сама `Grid`**, що й форма, `commit_grid`/
  `services::submission_grid`. Kvid → зовсім інша форма даних (`StaffingRow`, 01 §4) → окрема
  проста `StaffingTable`, без undo/автозбереження → `commit_staffing` (пише `staffing_snapshot`/
  `_metric` напряму, не через `submission_grid`). Ivs — ОБИДВІ форми з одного файлу: `staffing`→
  `InstructorStaffingTable`, `groups`→ ТОЙ САМИЙ `Grid` (стажування/курси — "рядок групи" з
  `training_kind='internship'`/`'special'+course_id`), `Ctrl+Enter` комітить обидва окремими
  викликами (не атомарно). Terminy — теж "рядок групи", три `training_kind` (bzvp/special/
  adaptation) з ОДНОГО файлу, той самий `Grid`, нової гілки UI не треба.
- **VchArchive (Етап 6) — той самий `Grid`, ІНШИЙ `submission.source_type`**: `FileKind::
  is_archive()` → `source_type='archive_seed'` (окреме тріо `get_archive_draft`/`save_archive_draft`/
  `commit_archive_grid` у `server.rs`, не runtime-параметр — джерело типізоване константою на
  клієнті). Без автозбереження й без `submission_id`-неперервності (коміт завжди з `None` —
  одноразовий перенос, чернетка-в-часі не потрібна). Статус превʼю рахує рядки з нерозпізнаною
  частиною — "звіт переносу" роадмапу Етапу 6; сам коміт лишається все-або-нічого.
- Файл читається в байти на клієнті (`File::array_buffer()`, без multipart) і йде як `Vec<u8>`
  у `#[server]`-аргументі.
- Дублює частину каркасу `training_form/mod.rs` (автозбереження, undo/redo, гарячі клавіші) —
  свідомо, поки не з'явиться третій споживач `widgets::group_grid` з тим самим патерном сторінки
  (07 §1: правило двох; VchArchive — той самий споживач, не новий).

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `ImportPage`, `ImportBody`, `StaffingTable`/`StaffingTableRow` (Kvid), `InstructorStaffingTable`/`InstructorStaffingTableRow` (Ivs) | `pages/CLAUDE.md` → `routes.rs` |
| `server.rs` | `parse_*_file` (6 типів), `commit_staffing`/`commit_instructor_staffing`, `get_draft`/`save_draft`/`commit_grid` (`table`), `get_archive_draft`/`save_archive_draft`/`commit_archive_grid` (`archive_seed`) — тонкі обгортки над `services::submission_grid` | `mod.rs` |
