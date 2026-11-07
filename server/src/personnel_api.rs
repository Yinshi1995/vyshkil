//! HTTP-шар модуля `personnel` (рішення `.claude/decisions/personnel-named-persons.md`).
//!
//! ВИМКНЕНИЙ ЗА ЗАМОВЧУВАННЯМ: доки не задано `PERSONNEL_ENABLED=1`, кожен ендпоінт відповідає 404,
//! ніби його не існує (прапорець читається на кожен виклик — вмикається без перезбірки, як `PASSWORD_LOGIN`).
//! Права — лише через `policy::can_manage_personnel`. Помилки й логи не містять значень полів особи.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get},
    Json, Router,
};
use sea_orm::TransactionTrait;

use app::backend::{policy, repo};
use app::backend::repo::personnel::NewPerson;

use crate::state::AppState;

/// Чи ввімкнено модуль особових даних (дефолт — ні).
pub fn enabled() -> bool {
    std::env::var("PERSONNEL_ENABLED").map(|v| v.trim() == "1").unwrap_or(false)
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/features", get(features_handler))
        .route("/api/data/events/:event_id/persons", get(list_handler).post(add_handler))
        .route("/api/data/events/:event_id/persons/:person_id", delete(unlink_handler))
}

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "error": msg })))
}

/// Спільна перевірка: прапорець → автентифікація → право. Повертає тег актора для журналу (без ПІБ).
async fn guard(state: &AppState, headers: &HeaderMap) -> Result<(i32, String), (StatusCode, Json<serde_json::Value>)> {
    if !enabled() {
        return Err(err(StatusCode::NOT_FOUND, "Не знайдено"));
    }
    let user = crate::api::require_auth(&state.db, headers)
        .await
        .map_err(|s| err(s, "Не авторизовано"))?;
    let actor = user.actor.ok_or_else(|| err(StatusCode::FORBIDDEN, "Недостатньо прав"))?;
    if !policy::can_manage_personnel(actor) {
        return Err(err(StatusCode::FORBIDDEN, "Недостатньо прав"));
    }
    Ok((user.user_id, policy::session_tag(actor)))
}

/// Які можливості ввімкнено — читає React, щоб приховати інтерфейс. Доступно й без входу:
/// відповідь не містить даних, лише булеві прапорці.
async fn features_handler() -> impl IntoResponse {
    Json(serde_json::json!({ "personnel": enabled() }))
}

async fn list_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(event_id): Path<i32>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let (_uid, tag) = guard(&state, &headers).await?;
    if repo::personnel::event_group_id(&state.db, event_id)
        .await
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Помилка сервера"))?
        .is_none()
    {
        return Err(err(StatusCode::NOT_FOUND, "Події не існує"));
    }
    let rows = repo::personnel::list_event_persons(&state.db, event_id, &tag)
        .await
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Помилка сервера"))?;
    Ok(Json(rows))
}

async fn add_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(event_id): Path<i32>,
    Json(body): Json<NewPerson>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let (uid, tag) = guard(&state, &headers).await?;
    let txn = state
        .db
        .begin()
        .await
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Помилка сервера"))?;
    let (person, created) = match repo::personnel::link_person_to_event(&txn, event_id, &body, Some(uid), &tag).await {
        Ok(v) => v,
        // `Custom` — наші повідомлення без значень полів (валідація, ліміт, дубль, немає події).
        Err(sea_orm::DbErr::Custom(msg)) => {
            let status = if msg == "Події не існує" { StatusCode::NOT_FOUND } else { StatusCode::UNPROCESSABLE_ENTITY };
            return Err(err(status, &msg));
        }
        Err(_) => return Err(err(StatusCode::INTERNAL_SERVER_ERROR, "Помилка сервера")),
    };
    txn.commit()
        .await
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Помилка сервера"))?;
    let status = if created { StatusCode::CREATED } else { StatusCode::OK };
    Ok((status, Json(person)))
}

async fn unlink_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((event_id, person_id)): Path<(i32, i32)>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let (_uid, tag) = guard(&state, &headers).await?;
    let removed = repo::personnel::unlink_person(&state.db, event_id, person_id, &tag)
        .await
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Помилка сервера"))?;
    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(err(StatusCode::NOT_FOUND, "Зв'язку не існує"))
    }
}
