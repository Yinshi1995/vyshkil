# app/src/pages/import — превʼю імпорту (`/import`, 03, Етап 5)

- Можна: `services`, `widgets`, `hooks`, `types`, `domain`, `backend` (лише з `server.rs`, під `ssr`).
  Не можна: інші сторінки.
- `server.rs` оголошений без `pub` (`mod server;`) — приватний для цієї сторінки.
- Права: `viewer` не імпортує; `org_editor` — комітиться лише за власну організацію (той самий
  `policy::can_edit_org` у `commit_grid_impl`, `services/submission_grid.rs`, що й у формі).
- Поки лише "Фах" (`backend::import::fah`) — 4 інші файли з критерію готовності Етапу 5
  (БпС/КВід/ІВС/Терміни) мають ІНШУ структуру кожен (див. `backend/import/CLAUDE.md`), додаються
  окремими кроками: очікуй, що ця сторінка переросте у вибір типу файлу, коли додасться другий.
- Файл читається в байти на клієнті (`File::array_buffer()`, без multipart — файли малі) і
  надсилається як `Vec<u8>` в `#[server]`-аргументі.
- Дублює частину каркасу `training_form/mod.rs` (автозбереження, undo/redo, гарячі клавіші) —
  свідомо, не винесено в спільний хук: обсяг виправдовує дублювання каркасу сторінки, поки не
  з'явиться третій споживач `widgets::group_grid` з тим самим патерном (07 §1: правило двох).

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `ImportPage` — завантаження файлу, стан сітки, автозбереження, коміт | `pages/CLAUDE.md` → `routes.rs` |
| `server.rs` | `parse_fah_file`, `get_draft`/`save_draft`/`commit_grid` (тонкі обгортки над `services::submission_grid`) | `mod.rs` |
