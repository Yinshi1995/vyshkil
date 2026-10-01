# app/src/types — DTO і спільні типи, що їздять клієнт↔сервер

- Можна: `serde`. Не можна: `leptos`, `sea_orm`, `tokio` — компілюється і в WASM, і на сервері
  без винятків (07 §2.3).
- Новий DTO для сервер-функції → сюди, не поруч зі server fn у `pages/`/`services/`.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `actor.rs` | `Actor`/`Role` — org_id + роль, `Serialize`/`Deserialize` | `layout`, `pages/*`, `backend::policy` |
| `org.rs` | `OrgSearchResult`, `OrgTreeRow`, `OrgDetail` | `backend/repo/orgs.rs`, `pages/home`, `pages/org_detail` |
| `dictionaries.rs` | `EquipmentVosHint`, `DictionaryEntry`, `DictionariesOverview`, `LearnedAlias` | `backend/repo/dictionaries.rs`, `pages/vos_lookup`, `pages/dictionaries` |
| `submission.rs` | `GroupFormRow`, `DraftPayload`/`DraftState` (02 §6), `VosPositionCourseHint`, `TrainingSiteOption`, `CommitOutcome` | `backend/repo/groups.rs`, `backend/repo/submissions.rs`, `pages/training_form` |
| `staffing.rs` | `StaffingRow` (КВід) / `InstructorStaffingRow` (ІВС) — укомплектованість (01 §4): org + кілька чисел, РІЗНІ набори метрик, не group-подібні дані | `backend/repo/staffing.rs`, `backend/repo/imports_kvid.rs`/`imports_ivs.rs`, `pages/import` |
| `reconciliation.rs` | `DiscrepancyRow` — рядок екрана розбіжностей (04 §4, Етап 8 зріз 1) | `backend/repo/reconciliation.rs`, `services/reconciliation.rs`, `pages/discrepancies` |
| `notification.rs` | DTO внутрішніх сповіщень (Етап 8 зріз 4, 04 §5) | `backend/repo/notifications.rs`, `services/notifications.rs`, `layout` |
