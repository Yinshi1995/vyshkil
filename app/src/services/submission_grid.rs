//! Спільна логіка чернетки/фіксації сітки груп (02 §5-6) — однакова для форми (`pages::
//! training_form`, `source_type='form'`) і імпорту (`pages::import`, `source_type='table'`, Етап 5).
//! Не `#[server]` самі по собі: кожна сторінка загортає ці функції у власну `#[server]`-функцію
//! (макрос генерує окремий тип на кожен виклик, спільну сигнатуру з параметром не заведеш) — див.
//! `pages/training_form/server.rs`/`pages/import/server.rs`.

use crate::types::actor::Actor;
use crate::types::submission::{CommitOutcome, DraftPayload, DraftState};

pub async fn get_draft_impl(
    actor: Option<Actor>,
    source_type: &str,
) -> Result<Option<DraftState>, String> {
    use crate::backend::repo;

    let Some(actor) = actor else { return Ok(None) };
    let db = leptos::prelude::expect_context::<sea_orm::DatabaseConnection>();
    repo::submissions::latest_draft_for_org(&db, actor.org_id, source_type)
        .await
        .map_err(|e| e.to_string())
}

pub async fn save_draft_impl(
    actor: Option<Actor>,
    submission_id: Option<i32>,
    source_type: &str,
    payload: DraftPayload,
) -> Result<i32, String> {
    use crate::backend::{db, policy, repo};

    let actor = actor.ok_or_else(|| "оберіть актора вгорі".to_string())?;
    if actor.role == policy::Role::Viewer {
        return Err("перегляд не зберігає чернетки".to_string());
    }

    let txn = db::actor_transaction(actor).await.map_err(|e| e.to_string())?;
    let id = repo::submissions::save_draft(&txn, submission_id, actor.org_id, source_type, &payload)
        .await
        .map_err(|e| e.to_string())?;
    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(id)
}

pub async fn commit_grid_impl(
    actor: Option<Actor>,
    submission_id: Option<i32>,
    source_type: &str,
    payload: DraftPayload,
) -> Result<CommitOutcome, String> {
    use crate::backend::{db, policy, repo};

    let actor = actor.ok_or_else(|| "оберіть актора вгорі".to_string())?;
    if actor.role == policy::Role::Viewer {
        return Err("перегляд не зберігає дані".to_string());
    }
    if payload.rows.is_empty() {
        return Err("немає жодного рядка для збереження".to_string());
    }

    // as_of_date приходить з <input type="date"> -- вже ISO, парсимо напряму (без евристики
    // року з domain::dates::parse_date, та для рядків без явного року).
    let as_of = chrono::NaiveDate::parse_from_str(&payload.as_of_date, "%Y-%m-%d")
        .map_err(|_| "«станом на»: неможлива дата".to_string())?;

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

    let txn = db::actor_transaction(actor).await.map_err(|e| e.to_string())?;
    let effective_submission_id =
        repo::submissions::save_draft(&txn, submission_id, actor.org_id, source_type, &payload)
            .await
            .map_err(|e| e.to_string())?;
    let group_ids = repo::groups::commit_group_rows(&txn, &validated, effective_submission_id)
        .await
        .map_err(|e| e.to_string())?;
    repo::submissions::mark_committed(&txn, effective_submission_id)
        .await
        .map_err(|e| e.to_string())?;
    txn.commit().await.map_err(|e| e.to_string())?;

    Ok(CommitOutcome::Committed { group_ids })
}
