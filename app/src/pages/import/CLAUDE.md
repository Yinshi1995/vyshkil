# app/src/pages/import — превʼю імпорту (`/import`, 03, Етап 5)

- Можна: `services`, `widgets`, `hooks`, `types`, `domain`, `backend` (лише з `server.rs`, під `ssr`).
  Не можна: інші сторінки.
- `server.rs` оголошений без `pub` (`mod server;`) — приватний для цієї сторінки.
- Права: `viewer` не імпортує; `org_editor` — комітиться лише за власну організацію (той самий
  `policy::can_edit_org` у `commit_grid_impl`, `services/submission_grid.rs`, що й у формі).
- Три типи файлу зараз (Фах, БпС, КВід) — перемикач `<select>` у `ImportBody`, `FileKind` вирішує,
  який `parse_*_file` викликати. ІВС/Терміни з критерію готовності Етапу 5 ще не додані (ІНША
  структура кожен, див. `backend/import/CLAUDE.md`) — додаються окремими кроками.
- **Fah/Bps vs Kvid — дві різні форми даних, дві різні гілки UI**: `FileKind::is_staffing()`
  розводить їх. Fah/Bps → `GroupFormRow` → `widgets::group_grid::Grid` (та сама сітка, що й
  форма) → `commit_grid`/`services::submission_grid`, з draft-автозбереженням і undo/redo. Kvid →
  `StaffingRow` (01 §4, org + сім чисел, БЕЗ ВОС/дат/воронки) → окрема проста `StaffingTable`
  (плейн `<input type="number">`, без undo/автозбереження — задокументоване спрощення, менший
  обсяг файлу (17 рядків) не виправдовує той самий каркас) → `commit_staffing`, який сам пише
  `submission(status='committed')` + `staffing_snapshot`/`_metric` напряму (не через
  `services::submission_grid` — там усе заточено під `GroupFormRow`).
- Файл читається в байти на клієнті (`File::array_buffer()`, без multipart — файли малі) і
  надсилається як `Vec<u8>` в `#[server]`-аргументі.
- Дублює частину каркасу `training_form/mod.rs` (автозбереження, undo/redo, гарячі клавіші) —
  свідомо, не винесено в спільний хук: обсяг виправдовує дублювання каркасу сторінки, поки не
  з'явиться третій споживач `widgets::group_grid` з тим самим патерном (07 §1: правило двох).

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `ImportPage`, `ImportBody` (Фах/БпС стан), `StaffingTable`/`StaffingTableRow` (Kvid превʼю) | `pages/CLAUDE.md` → `routes.rs` |
| `server.rs` | `parse_fah_file`/`parse_bps_file`/`parse_kvid_file`, `commit_staffing`, `get_draft`/`save_draft`/`commit_grid` (тонкі обгортки над `services::submission_grid`) | `mod.rs` |
