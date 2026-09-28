use leptos::prelude::*;

use crate::types::actor::Actor;
use crate::types::submission::{
    CommitOutcome, DraftPayload, DraftState, TrainingSiteOption, VosPositionCourseHint,
};

/// Чернетка організації актора, якщо є (02 §6: відновлення після обриву зв'язку/закритої вкладки).
#[server(GetDraft, "/api")]
pub async fn get_draft(actor: Option<Actor>) -> Result<Option<DraftState>, ServerFnError> {
    use crate::backend::repo;

    let Some(actor) = actor else { return Ok(None) };
    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::submissions::latest_draft_for_org(&db, actor.org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Автозбереження сітки (02 §6) — кожні кілька секунд з клієнта. `viewer` не редагує (01 §6),
/// тож і чернетку не зберігає.
#[server(SaveDraft, "/api")]
pub async fn save_draft(
    actor: Option<Actor>,
    submission_id: Option<i32>,
    payload: DraftPayload,
) -> Result<i32, ServerFnError> {
    use crate::backend::{db, policy, repo};

    let actor = actor.ok_or_else(|| ServerFnError::new("оберіть актора вгорі"))?;
    if actor.role == policy::Role::Viewer {
        return Err(ServerFnError::new("перегляд не зберігає чернетки"));
    }

    let txn = db::actor_transaction(actor).await.map_err(|e| ServerFnError::new(e.to_string()))?;
    let id = repo::submissions::save_draft(&txn, submission_id, actor.org_id, &payload)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    txn.commit().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(id)
}

/// Одне поле "ВОС / посада / курс" (02 §3) — не org-scoped, як і `equipment_vos_hint`.
#[server(SearchVosPositionCourse, "/api")]
pub async fn search_vos_position_course(
    query: String,
) -> Result<Vec<VosPositionCourseHint>, ServerFnError> {
    use crate::backend::repo;

    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::groups::search_vos_position_course(&db, &query)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Майданчики навчання обраної частини (02 §1 колонка 5).
#[server(GetTrainingSites, "/api")]
pub async fn get_training_sites(org_id: i32) -> Result<Vec<TrainingSiteOption>, ServerFnError> {
    use crate::backend::repo;

    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::groups::training_site_options(&db, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Фіксація сітки (`Ctrl+Enter`, 02 §5) — усе-або-нічого: валідує кожен рядок (дати, порядок
/// воронки, обов'язкові поля) і право редагування (`org_editor` — лише власна організація, 01 §6)
/// ДО запису; перша помилка зупиняє коміт і повертається клітинкою, яку сітка підсвічує.
#[server(CommitGrid, "/api")]
pub async fn commit_grid(
    actor: Option<Actor>,
    submission_id: Option<i32>,
    payload: DraftPayload,
) -> Result<CommitOutcome, ServerFnError> {
    use crate::backend::{db, policy, repo};

    let actor = actor.ok_or_else(|| ServerFnError::new("оберіть актора вгорі"))?;
    if actor.role == policy::Role::Viewer {
        return Err(ServerFnError::new("перегляд не зберігає дані"));
    }
    if payload.rows.is_empty() {
        return Err(ServerFnError::new("немає жодного рядка для збереження"));
    }

    // as_of_date приходить з <input type="date"> -- вже ISO, парсимо напряму (без евристики
    // року з domain::dates::parse_date, та для рядків без явного року).
    let as_of = chrono::NaiveDate::parse_from_str(&payload.as_of_date, "%Y-%m-%d")
        .map_err(|_| ServerFnError::new("«станом на»: неможлива дата"))?;

    let mut validated = Vec::with_capacity(payload.rows.len());
    for (index, row) in payload.rows.iter().enumerate() {
        if let Some(sender_org_id) = row.sender_org_id {
            if !policy::can_edit_org(actor, sender_org_id) {
                return Ok(CommitOutcome::ValidationFailed {
                    row_index: index,
                    field: "sender_org_id".to_string(),
                    message: "немає права вносити дані за цю частину".to_string(),
                });
            }
        }
        match repo::groups::validate_row(row, as_of) {
            Ok(v) => validated.push((row.clone(), v)),
            Err((field, message)) => {
                return Ok(CommitOutcome::ValidationFailed { row_index: index, field, message });
            }
        }
    }

    let txn = db::actor_transaction(actor).await.map_err(|e| ServerFnError::new(e.to_string()))?;
    let effective_submission_id =
        repo::submissions::save_draft(&txn, submission_id, actor.org_id, &payload)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
    let group_ids = repo::groups::commit_group_rows(&txn, &validated, effective_submission_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    repo::submissions::mark_committed(&txn, effective_submission_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    txn.commit().await.map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(CommitOutcome::Committed { group_ids })
}
