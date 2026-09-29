# app/src/pages/import — превʼю імпорту (`/import`, 03, Етап 5)

- Можна: `services`, `widgets`, `hooks`, `types`, `domain`, `backend` (лише з `server.rs`, під `ssr`).
  Не можна: інші сторінки.
- `server.rs` оголошений без `pub` (`mod server;`) — приватний для цієї сторінки.
- Права: `viewer` не імпортує; `org_editor` — комітиться лише за власну організацію (той самий
  `policy::can_edit_org` у `commit_grid_impl`, `services/submission_grid.rs`, що й у формі).
- Чотири типи файлу зараз (Фах, БпС, КВід, ІВС) — перемикач `<select>` у `ImportBody`, `FileKind`
  вирішує, який `parse_*_file` викликати. Терміни з критерію готовності Етапу 5 ще не додано (3
  паралельні блоки з вертикально злитими комірками — найскладніша структура, `backend/import/
  CLAUDE.md`) — окремим кроком.
- **Fah/Bps vs Kvid vs Ivs — три різні гілки UI, `FileKind::is_staffing()` розводить лише перші
  дві**: Fah/Bps → `GroupFormRow` → `widgets::group_grid::Grid` (та сама сітка, що й форма) →
  `commit_grid`/`services::submission_grid`, з draft-автозбереженням і undo/redo. Kvid →
  `StaffingRow` (01 §4, org + сім чисел, БЕЗ ВОС/дат/воронки) → окрема проста `StaffingTable`
  (плейн `<input type="number">`, без undo/автозбереження — задокументоване спрощення, менший
  обсяг файлу (17 рядків) не виправдовує той самий каркас) → `commit_staffing`, який сам пише
  `submission(status='committed')` + `staffing_snapshot`/`_metric` напряму (не через
  `services::submission_grid` — там усе заточено під `GroupFormRow`). **Ivs — ОБИДВІ форми
  одразу з одного файлу** (`backend/import/ivs.rs`: одна `xlsx` дає і укомплектованість, і
  стажування/курси): `parse_ivs_file` повертає `(Vec<InstructorStaffingRow>, Vec<GroupFormRow>)`
  — перше йде в `InstructorStaffingTable` (той самий патерн, що й `StaffingTable`, інший набір
  метрик), друге — у ТОЙ САМИЙ `Grid`, що й Фах/БпС (стажування/курси — це "рядок групи" з
  `training_kind='internship'`/`'special'+course_id` замість `vos`/`position`). `Ctrl+Enter`
  комітить ОБИДВА окремими викликами (`commit_instructor_staffing` + `commit_grid`) — не
  атомарно разом: часткова невдача одного не блокує інший, той самий рівень, що й "спробуй ще
  раз" для решти імпортів сторінки.
- Файл читається в байти на клієнті (`File::array_buffer()`, без multipart — файли малі) і
  надсилається як `Vec<u8>` в `#[server]`-аргументі.
- Дублює частину каркасу `training_form/mod.rs` (автозбереження, undo/redo, гарячі клавіші) —
  свідомо, не винесено в спільний хук: обсяг виправдовує дублювання каркасу сторінки, поки не
  з'явиться третій споживач `widgets::group_grid` з тим самим патерном (07 §1: правило двох).

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `ImportPage`, `ImportBody` (Фах/БпС/Ivs-груп стан), `StaffingTable`/`StaffingTableRow` (Kvid превʼю), `InstructorStaffingTable`/`InstructorStaffingTableRow` (Ivs превʼю) | `pages/CLAUDE.md` → `routes.rs` |
| `server.rs` | `parse_fah_file`/`parse_bps_file`/`parse_kvid_file`/`parse_ivs_file`, `commit_staffing`/`commit_instructor_staffing`, `get_draft`/`save_draft`/`commit_grid` (тонкі обгортки над `services::submission_grid`) | `mod.rs` |
