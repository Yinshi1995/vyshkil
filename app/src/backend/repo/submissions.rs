//! SQL для чернеток (`submission`, 01 §5, 02 §6) — спільна для форми (`source_type='form'`) і
//! імпорту (`'table'`, Етап 5): обидва однаково зберігають/відновлюють чернетку сітки, різниться
//! лише джерело подання. Фіксація (draft → committed, запис у `training_group`/`group_event`) —
//! `repo::groups::commit_group_rows`, викликається разом з `mark_committed` у транзакції з
//! `pages/training_form/server.rs` / `pages/import/server.rs`.

use crate::types::submission::{DraftPayload, DraftState};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};

// draft_payload читається/пишеться як текст (::jsonb / ::text) через ручний serde_json::to_string/
// from_str, а не через sea-orm Value::Json -- щоб не тягнути sea-orm feature "with-json" (і з ним
// mysql/sqlite/rsa-залежності через уніфікацію фіч у Cargo.lock) заради одного jsonb-стовпця.
#[derive(FromQueryResult)]
struct DraftRow {
    id: i32,
    payload: String,
    updated_at: String,
}

fn row_to_state(row: DraftRow) -> Result<DraftState, DbErr> {
    let payload: DraftPayload = serde_json::from_str(&row.payload)
        .map_err(|e| DbErr::Custom(format!("draft_payload не розбирається: {e}")))?;
    Ok(DraftState { submission_id: row.id, payload, updated_at: row.updated_at })
}

/// Незавершена чернетка організації, якщо є (найновіша) — для відновлення сітки при відкритті
/// форми після обриву зв'язку/закритої вкладки (02 §6).
pub async fn latest_draft_for_org(
    db: &DatabaseConnection,
    reporting_org_id: i32,
    source_type: &str,
) -> Result<Option<DraftState>, DbErr> {
    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, draft_payload::text AS payload, \
                to_char(updated_at, 'YYYY-MM-DD\"T\"HH24:MI:SS') AS updated_at \
         FROM submission \
         WHERE reporting_org_id = $1 AND source_type = $2 AND status = 'draft' \
         ORDER BY updated_at DESC LIMIT 1",
        [reporting_org_id.into(), source_type.into()],
    );
    let row = DraftRow::find_by_statement(stmt).one(db).await?;
    row.map(row_to_state).transpose()
}

/// Автозбереження (02 §6): новий рядок при першому збереженні сітки, інакше `UPDATE` тієї самої
/// чернетки (тому й потрібен `submission_id`, який клієнт отримує з першої відповіді і надсилає
/// далі). Транзакція з `SET LOCAL app.actor` — виклик обгортає `db::actor_transaction`.
pub async fn save_draft(
    db: &impl ConnectionTrait,
    submission_id: Option<i32>,
    reporting_org_id: i32,
    source_type: &str,
    payload: &DraftPayload,
) -> Result<i32, DbErr> {
    let payload_json = serde_json::to_string(payload)
        .map_err(|e| DbErr::Custom(format!("не вдалось серіалізувати чернетку: {e}")))?;

    if let Some(id) = submission_id {
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "UPDATE submission SET draft_payload = $1::jsonb, as_of_date = $2::date, updated_at = now() \
             WHERE id = $3 AND status = 'draft'",
            [payload_json.clone().into(), payload.as_of_date.clone().into(), id.into()],
        ))
        .await?;

        // Чернетку могли вже зафіксувати (напр. в іншій вкладці) -- тоді UPDATE зачепив 0 рядків
        // і треба почати нову, а не мовчки загубити автозбереження.
        let still_draft = db
            .query_one(Statement::from_sql_and_values(
                db.get_database_backend(),
                "SELECT id FROM submission WHERE id = $1 AND status = 'draft'",
                [id.into()],
            ))
            .await?;
        if still_draft.is_some() {
            return Ok(id);
        }
    }

    #[derive(FromQueryResult)]
    struct NewId {
        id: i32,
    }
    let row = NewId::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO submission (source_type, reporting_org_id, as_of_date, status, draft_payload) \
         VALUES ($1, $2, $3::date, 'draft', $4::jsonb) RETURNING id",
        [
            source_type.into(),
            reporting_org_id.into(),
            payload.as_of_date.clone().into(),
            payload_json.into(),
        ],
    ))
    .one(db)
    .await?
    .ok_or_else(|| DbErr::Custom("INSERT submission не повернув id".into()))?;

    Ok(row.id)
}

/// Переводить чернетку у `committed` і чистить `draft_payload` (більше не потрібен -- канонічні
/// дані вже в `training_group`/`group_event`). Викликається в тій самій транзакції, що й
/// `repo::groups::commit_group_rows`.
pub async fn mark_committed(db: &impl ConnectionTrait, submission_id: i32) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE submission SET status = 'committed', draft_payload = NULL, updated_at = now() \
         WHERE id = $1 AND status = 'draft'",
        [submission_id.into()],
    ))
    .await?;
    Ok(())
}
