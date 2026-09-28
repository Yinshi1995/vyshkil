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

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `TrainingFormPage` — стан сітки, автозбереження, undo/redo, глобальні гарячі клавіші, шпаргалка, командна палітра | `pages/CLAUDE.md` → `routes.rs` |
| `server.rs` | `get_draft`/`save_draft`/`commit_grid`, `search_vos_position_course`, `get_training_sites` | `mod.rs`, `components/` |
| `components/` | `Grid` (своя карта) | `mod.rs` |
