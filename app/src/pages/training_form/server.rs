use leptos::prelude::*;

use crate::types::actor::Actor;
use crate::types::submission::{CommitOutcome, DraftPayload, DraftState};

const SOURCE_TYPE: &str = "form";

/// Чернетка організації актора, якщо є (02 §6: відновлення після обриву зв'язку/закритої вкладки).
#[server(GetDraft, "/api")]
pub async fn get_draft(actor: Option<Actor>) -> Result<Option<DraftState>, ServerFnError> {
    use crate::services::submission_grid::get_draft_impl;

    get_draft_impl(actor, SOURCE_TYPE).await.map_err(ServerFnError::new)
}

/// Автозбереження сітки (02 §6) — кожні кілька секунд з клієнта. `viewer` не редагує (01 §6),
/// тож і чернетку не зберігає.
#[server(SaveDraft, "/api")]
pub async fn save_draft(
    actor: Option<Actor>,
    submission_id: Option<i32>,
    payload: DraftPayload,
) -> Result<i32, ServerFnError> {
    use crate::services::submission_grid::save_draft_impl;

    save_draft_impl(actor, submission_id, SOURCE_TYPE, payload).await.map_err(ServerFnError::new)
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
    use crate::services::submission_grid::commit_grid_impl;

    commit_grid_impl(actor, submission_id, SOURCE_TYPE, payload).await.map_err(ServerFnError::new)
}
