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
