use std::convert::Infallible;
use std::time::Duration;

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    routing::{get, post},
    Json, Router,
};
use sea_orm::{ConnectionTrait, DatabaseConnection, FromQueryResult, Statement};
use serde::{Deserialize, Serialize};

use crate::api::require_auth;
use crate::state::{AppState, ChatBroadcastMsg};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Serialize, FromQueryResult)]
struct ChatRoom {
    id: i32,
    name: String,
    kind: String,
    emoji: Option<String>,
    unread_count: Option<i64>,
    last_message_body: Option<String>,
    last_message_at: Option<String>,
    member_count: Option<i64>,
    dm_avatar: Option<String>,
}

#[derive(Serialize, FromQueryResult)]
struct ChatMessage {
    id: i32,
    room_id: i32,
    sender_id: i32,
    sender_label: Option<String>,
    sender_callsign: Option<String>,
    sender_avatar: Option<String>,
    sender_role: Option<String>,
    kind: String,
    body: String,
    media_url: Option<String>,
    media_mime: Option<String>,
    media_duration_sec: Option<f32>,
    reply_to_id: Option<i32>,
    reply_preview: Option<String>,
    created_at: String,
    updated_at: Option<String>,
}

#[derive(Deserialize)]
struct SendMessageReq {
    body: Option<String>,
    kind: Option<String>,
    media_url: Option<String>,
    media_mime: Option<String>,
    media_duration_sec: Option<f32>,
    reply_to_id: Option<i32>,
}

#[derive(Deserialize)]
struct MessageQuery {
    before: Option<i32>,
    limit: Option<i32>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn list_rooms(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ChatRoom>>, StatusCode> {
    let user = require_auth(&state.db, &headers).await?;

    let rooms = ChatRoom::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        r#"SELECT
                cr.id,
                CASE WHEN cr.kind = 'direct' THEN
                    COALESCE(
                        (SELECT COALESCE(ua2.callsign, ua2.display_name, ua2.login)
                         FROM chat_room_member crm3
                         JOIN user_account ua2 ON ua2.id = crm3.user_id
                         WHERE crm3.room_id = cr.id AND crm3.user_id <> $1
                         LIMIT 1),
                        cr.name
                    )
                ELSE cr.name END AS name,
                cr.kind, cr.emoji,
                (SELECT COUNT(*) FROM chat_message cm
                 WHERE cm.room_id = cr.id
                   AND cm.deleted_at IS NULL
                   AND cm.created_at > COALESCE(crm.last_read_at, '1970-01-01'::timestamptz)
                )::bigint AS unread_count,
                (SELECT CASE cm2.kind
                    WHEN 'voice' THEN '🎤 Голосове'
                    WHEN 'image' THEN '📷 Фото'
                    WHEN 'video' THEN '📹 Відео'
                    WHEN 'file' THEN '📎 Файл'
                    WHEN 'system' THEN '📋 ' || LEFT(cm2.body, 50)
                    ELSE cm2.body
                 END
                 FROM chat_message cm2
                 WHERE cm2.room_id = cr.id AND cm2.deleted_at IS NULL
                 ORDER BY cm2.created_at DESC LIMIT 1
                ) AS last_message_body,
                (SELECT to_char(cm3.created_at, 'YYYY-MM-DD"T"HH24:MI:SS') FROM chat_message cm3
                 WHERE cm3.room_id = cr.id AND cm3.deleted_at IS NULL
                 ORDER BY cm3.created_at DESC LIMIT 1
                ) AS last_message_at,
                (SELECT COUNT(*) FROM chat_room_member crm2
                 WHERE crm2.room_id = cr.id
                )::bigint AS member_count,
                CASE WHEN cr.kind = 'direct' THEN
                    (SELECT ua3.avatar_path
                     FROM chat_room_member crm4
                     JOIN user_account ua3 ON ua3.id = crm4.user_id
                     WHERE crm4.room_id = cr.id AND crm4.user_id <> $1
                     LIMIT 1)
                END AS dm_avatar
            FROM chat_room cr
            JOIN chat_room_member crm ON crm.room_id = cr.id AND crm.user_id = $1
            WHERE cr.is_active = TRUE
            ORDER BY last_message_at DESC NULLS LAST, cr.id"#,
        [user.user_id.into()],
    ))
    .all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(rooms))
}

async fn room_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(room_id): Path<i32>,
    Query(q): Query<MessageQuery>,
) -> Result<Json<Vec<ChatMessage>>, StatusCode> {
    let user = require_auth(&state.db, &headers).await?;

    // Verify membership
    verify_membership(&state.db, room_id, user.user_id).await?;

    let limit = q.limit.unwrap_or(50).min(100);
    let before_clause = if let Some(before) = q.before {
        format!("AND cm.id < {before}")
    } else {
        String::new()
    };

    let messages = ChatMessage::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        &format!(
            r#"SELECT
                cm.id, cm.room_id, cm.sender_id,
                COALESCE(ua.callsign, ua.display_name, ua.login) AS sender_label,
                ua.callsign AS sender_callsign,
                ua.avatar_path AS sender_avatar,
                (SELECT ur.role FROM user_role ur WHERE ur.user_id = ua.id
                 ORDER BY CASE ur.role WHEN 'admin' THEN 0 WHEN 'org_editor' THEN 1 ELSE 2 END
                 LIMIT 1) AS sender_role,
                cm.kind, cm.body,
                cm.media_url, cm.media_mime, cm.media_duration_sec,
                cm.reply_to_id,
                (SELECT LEFT(r.body, 80) FROM chat_message r WHERE r.id = cm.reply_to_id) AS reply_preview,
                to_char(cm.created_at, 'YYYY-MM-DD"T"HH24:MI:SS') AS created_at,
                CASE WHEN cm.updated_at > cm.created_at + interval '1 second'
                     THEN to_char(cm.updated_at, 'YYYY-MM-DD"T"HH24:MI:SS') END AS updated_at
            FROM chat_message cm
            JOIN user_account ua ON ua.id = cm.sender_id
            WHERE cm.room_id = $1
              AND cm.deleted_at IS NULL
              {before_clause}
            ORDER BY cm.created_at DESC
            LIMIT {limit}"#
        ),
        [room_id.into()],
    ))
    .all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Update last_read_at
    let _ = state.db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "UPDATE chat_room_member SET last_read_at = now() WHERE room_id = $1 AND user_id = $2",
        [room_id.into(), user.user_id.into()],
    )).await;

    Ok(Json(messages))
}

async fn send_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(room_id): Path<i32>,
    Json(req): Json<SendMessageReq>,
) -> Result<Json<ChatMessage>, StatusCode> {
    let user = require_auth(&state.db, &headers).await?;
    verify_membership(&state.db, room_id, user.user_id).await?;

    let kind = req.kind.as_deref().unwrap_or("text");
    let body = req.body.as_deref().unwrap_or("");

    if kind == "text" && body.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Insert message
    #[derive(FromQueryResult)]
    struct IdRow { id: i32 }

    let row = IdRow::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        r#"INSERT INTO chat_message (room_id, sender_id, kind, body, media_url, media_mime, media_duration_sec, reply_to_id)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING id"#,
        [
            room_id.into(),
            user.user_id.into(),
            kind.into(),
            body.into(),
            req.media_url.clone().map(|v| sea_orm::Value::String(Some(Box::new(v)))).unwrap_or(sea_orm::Value::String(None)),
            req.media_mime.clone().map(|v| sea_orm::Value::String(Some(Box::new(v)))).unwrap_or(sea_orm::Value::String(None)),
            req.media_duration_sec.map(|v| sea_orm::Value::Float(Some(v))).unwrap_or(sea_orm::Value::Float(None)),
            req.reply_to_id.map(|v| sea_orm::Value::Int(Some(v))).unwrap_or(sea_orm::Value::Int(None)),
        ],
    ))
    .one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let msg_id = row.id;

    // Broadcast to SSE listeners
    let _ = state.chat_tx.send(ChatBroadcastMsg { room_id, message_id: msg_id });

    // Fetch and return the full message
    let msg = ChatMessage::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        r#"SELECT
                cm.id, cm.room_id, cm.sender_id,
                COALESCE(ua.callsign, ua.display_name, ua.login) AS sender_label,
                ua.callsign AS sender_callsign,
                ua.avatar_path AS sender_avatar,
                (SELECT ur.role FROM user_role ur WHERE ur.user_id = ua.id
                 ORDER BY CASE ur.role WHEN 'admin' THEN 0 WHEN 'org_editor' THEN 1 ELSE 2 END
                 LIMIT 1) AS sender_role,
                cm.kind, cm.body,
                cm.media_url, cm.media_mime, cm.media_duration_sec,
                cm.reply_to_id,
                (SELECT LEFT(r.body, 80) FROM chat_message r WHERE r.id = cm.reply_to_id) AS reply_preview,
                to_char(cm.created_at, 'YYYY-MM-DD"T"HH24:MI:SS') AS created_at,
                CASE WHEN cm.updated_at > cm.created_at + interval '1 second'
                     THEN to_char(cm.updated_at, 'YYYY-MM-DD"T"HH24:MI:SS') END AS updated_at
            FROM chat_message cm
            JOIN user_account ua ON ua.id = cm.sender_id
            WHERE cm.id = $1"#,
        [msg_id.into()],
    ))
    .one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Update last_read_at for sender
    let _ = state.db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "UPDATE chat_room_member SET last_read_at = now() WHERE room_id = $1 AND user_id = $2",
        [room_id.into(), user.user_id.into()],
    )).await;

    Ok(Json(msg))
}

// ---------------------------------------------------------------------------
// Media upload
// ---------------------------------------------------------------------------

async fn upload_media(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user = require_auth(&state.db, &headers).await?;

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut content_type: Option<String> = None;
    let mut original_name: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        if field.name() == Some("file") {
            content_type = field.content_type().map(|s| s.to_string());
            original_name = field.file_name().map(|s| s.to_string());
            let bytes = field.bytes().await.map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
            // 20MB limit for media
            if bytes.len() > 20 * 1024 * 1024 {
                return Err(StatusCode::PAYLOAD_TOO_LARGE);
            }
            file_bytes = Some(bytes.to_vec());
        }
    }

    let bytes = file_bytes.ok_or(StatusCode::BAD_REQUEST)?;
    let ct = content_type.unwrap_or_else(|| "application/octet-stream".to_string());

    let ext = if ct.starts_with("image/png") { "png" }
        else if ct.starts_with("image/jpeg") || ct.starts_with("image/jpg") { "jpg" }
        else if ct.starts_with("image/webp") { "webp" }
        else if ct.starts_with("image/gif") { "gif" }
        else if ct.starts_with("video/mp4") { "mp4" }
        else if ct.starts_with("video/webm") { "webm" }
        else if ct.starts_with("audio/webm") { "webm" }
        else if ct.starts_with("audio/ogg") { "ogg" }
        else if ct.starts_with("audio/mpeg") { "mp3" }
        else if ct.starts_with("audio/mp4") { "m4a" }
        else if ct == "application/pdf" { "pdf" }
        else if ct == "application/vnd.openxmlformats-officedocument.wordprocessingml.document" { "docx" }
        else if ct == "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" { "xlsx" }
        else if ct == "application/vnd.openxmlformats-officedocument.presentationml.presentation" { "pptx" }
        else if ct == "application/msword" { "doc" }
        else if ct == "application/vnd.ms-excel" { "xls" }
        else if ct == "application/vnd.ms-powerpoint" { "ppt" }
        else {
            original_name.as_deref()
                .and_then(|n| n.rsplit('.').next())
                .unwrap_or("bin")
        };

    let media_dir = std::path::Path::new("data/chat-media");
    std::fs::create_dir_all(media_dir).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let filename = format!("u{}_{}.{}", user.user_id, ts, ext);
    let filepath = media_dir.join(&filename);
    std::fs::write(&filepath, &bytes).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let url = format!("/api/chat/media/{}", filename);

    Ok(Json(serde_json::json!({
        "url": url,
        "mime": ct,
        "filename": original_name,
        "size": bytes.len(),
    })))
}

async fn serve_media(
    Path(filename): Path<String>,
) -> impl IntoResponse {
    // Sanitize filename
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return (StatusCode::BAD_REQUEST, HeaderMap::new(), Vec::new());
    }

    let path = std::path::Path::new("data/chat-media").join(&filename);
    match std::fs::read(&path) {
        Ok(bytes) => {
            let mut headers = HeaderMap::new();
            let mime = if filename.ends_with(".png") { "image/png" }
                else if filename.ends_with(".jpg") || filename.ends_with(".jpeg") { "image/jpeg" }
                else if filename.ends_with(".webp") { "image/webp" }
                else if filename.ends_with(".gif") { "image/gif" }
                else if filename.ends_with(".mp4") { "video/mp4" }
                else if filename.ends_with(".webm") {
                    if bytes.len() > 4 && &bytes[0..4] == b"\x1a\x45\xdf\xa3" {
                        "video/webm"
                    } else {
                        "audio/webm"
                    }
                }
                else if filename.ends_with(".ogg") { "audio/ogg" }
                else if filename.ends_with(".mp3") { "audio/mpeg" }
                else if filename.ends_with(".m4a") { "audio/mp4" }
                else if filename.ends_with(".pdf") { "application/pdf" }
                else if filename.ends_with(".docx") { "application/vnd.openxmlformats-officedocument.wordprocessingml.document" }
                else if filename.ends_with(".xlsx") { "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" }
                else if filename.ends_with(".pptx") { "application/vnd.openxmlformats-officedocument.presentationml.presentation" }
                else if filename.ends_with(".doc") { "application/msword" }
                else if filename.ends_with(".xls") { "application/vnd.ms-excel" }
                else if filename.ends_with(".ppt") { "application/vnd.ms-powerpoint" }
                else { "application/octet-stream" };
            headers.insert(header::CONTENT_TYPE, mime.parse().unwrap());
            headers.insert(header::CACHE_CONTROL, "public, max-age=31536000, immutable".parse().unwrap());
            if !mime.starts_with("image/") && !mime.starts_with("video/") && !mime.starts_with("audio/") {
                headers.insert(
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}\"", filename).parse().unwrap(),
                );
            }
            (StatusCode::OK, headers, bytes)
        }
        Err(_) => (StatusCode::NOT_FOUND, HeaderMap::new(), Vec::new()),
    }
}

// ---------------------------------------------------------------------------
// SSE: real-time stream
// ---------------------------------------------------------------------------

async fn room_stream(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(room_id): Path<i32>,
) -> impl IntoResponse {
    let user = match require_auth(&state.db, &headers).await {
        Ok(u) => u,
        Err(status) => return (status, "").into_response(),
    };
    if verify_membership(&state.db, room_id, user.user_id).await.is_err() {
        return (StatusCode::FORBIDDEN, "not a member").into_response();
    }

    let mut rx = state.chat_tx.subscribe();
    let db = state.db.clone();

    let stream = async_stream::stream! {
        // Initial ping
        yield Ok::<_, Infallible>(Event::default().event("connected").data("ok"));

        loop {
            match tokio::time::timeout(Duration::from_secs(30), rx.recv()).await {
                Ok(Ok(msg)) => {
                    if msg.room_id != room_id {
                        continue;
                    }
                    // Fetch the full message
                    if let Ok(Some(full)) = ChatMessage::find_by_statement(Statement::from_sql_and_values(
                        sea_orm::DatabaseBackend::Postgres,
                        r#"SELECT
                            cm.id, cm.room_id, cm.sender_id,
                            COALESCE(ua.callsign, ua.display_name, ua.login) AS sender_label,
                            ua.callsign AS sender_callsign,
                            ua.avatar_path AS sender_avatar,
                            (SELECT ur.role FROM user_role ur WHERE ur.user_id = ua.id
                             ORDER BY CASE ur.role WHEN 'admin' THEN 0 WHEN 'org_editor' THEN 1 ELSE 2 END
                             LIMIT 1) AS sender_role,
                            cm.kind, cm.body,
                            cm.media_url, cm.media_mime, cm.media_duration_sec,
                            cm.reply_to_id,
                            (SELECT LEFT(r.body, 80) FROM chat_message r WHERE r.id = cm.reply_to_id) AS reply_preview,
                            to_char(cm.created_at, 'YYYY-MM-DD"T"HH24:MI:SS') AS created_at,
                            CASE WHEN cm.updated_at > cm.created_at + interval '1 second'
                                 THEN to_char(cm.updated_at, 'YYYY-MM-DD"T"HH24:MI:SS') END AS updated_at
                        FROM chat_message cm
                        JOIN user_account ua ON ua.id = cm.sender_id
                        WHERE cm.id = $1"#,
                        [msg.message_id.into()],
                    )).one(&db).await {
                        if let Ok(json) = serde_json::to_string(&full) {
                            yield Ok(Event::default().event("message").data(json));
                        }
                    }
                }
                Ok(Err(_)) => {
                    // Lagged — skip
                    continue;
                }
                Err(_) => {
                    // Timeout — send keepalive
                    yield Ok(Event::default().comment("keepalive"));
                }
            }
        }
    };

    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn verify_membership(db: &DatabaseConnection, room_id: i32, user_id: i32) -> Result<(), StatusCode> {
    #[derive(FromQueryResult)]
    struct Exists { exists: Option<bool> }

    let row = Exists::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT EXISTS(SELECT 1 FROM chat_room_member WHERE room_id = $1 AND user_id = $2) AS exists",
        [room_id.into(), user_id.into()],
    ))
    .one(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if row.map(|r| r.exists.unwrap_or(false)).unwrap_or(false) {
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

// ---------------------------------------------------------------------------
// Edit / Delete message (within 30 seconds, own messages only)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct EditMessageReq {
    body: String,
}

async fn edit_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(msg_id): Path<i32>,
    Json(req): Json<EditMessageReq>,
) -> Result<StatusCode, StatusCode> {
    let user = require_auth(&state.db, &headers).await?;

    #[derive(FromQueryResult)]
    struct MsgCheck { sender_id: i32, age_seconds: Option<f64> }

    let row = MsgCheck::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT sender_id, EXTRACT(EPOCH FROM (now() - created_at))::float8 AS age_seconds FROM chat_message WHERE id = $1 AND deleted_at IS NULL",
        [msg_id.into()],
    ))
    .one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    if row.sender_id != user.user_id {
        return Err(StatusCode::FORBIDDEN);
    }
    if row.age_seconds.unwrap_or(999.0) > 30.0 {
        return Err(StatusCode::GONE);
    }

    state.db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "UPDATE chat_message SET body = $1, updated_at = now() WHERE id = $2",
        [req.body.into(), msg_id.into()],
    ))
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn delete_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(msg_id): Path<i32>,
) -> Result<StatusCode, StatusCode> {
    let user = require_auth(&state.db, &headers).await?;

    #[derive(FromQueryResult)]
    struct MsgCheck { sender_id: i32 }

    let row = MsgCheck::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT sender_id FROM chat_message WHERE id = $1 AND deleted_at IS NULL",
        [msg_id.into()],
    ))
    .one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let is_own = row.sender_id == user.user_id;

    if !is_own {
        // Check if user is admin and sender belongs to their org tree
        let actor = user.actor.ok_or(StatusCode::FORBIDDEN)?;
        if !app::backend::policy::is_admin(actor) {
            return Err(StatusCode::FORBIDDEN);
        }

        #[derive(FromQueryResult)]
        struct InTree { in_tree: Option<bool> }

        let check = InTree::find_by_statement(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            r#"SELECT EXISTS(
                SELECT 1 FROM user_role ur
                JOIN subordination_closure sc ON sc.descendant_id = ur.org_id AND sc.ancestor_id = $1
                WHERE ur.user_id = $2
            ) AS in_tree"#,
            [actor.org_id.into(), row.sender_id.into()],
        ))
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if !check.map(|c| c.in_tree.unwrap_or(false)).unwrap_or(false) {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    state.db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "UPDATE chat_message SET deleted_at = now() WHERE id = $1",
        [msg_id.into()],
    ))
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// DM: find or create direct room
// ---------------------------------------------------------------------------

#[derive(Serialize, FromQueryResult)]
struct DmRoomResult {
    room_id: i32,
}

async fn open_dm(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(other_user_id): Path<i32>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user = require_auth(&state.db, &headers).await?;

    if other_user_id == user.user_id {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check target user exists
    #[derive(FromQueryResult)]
    struct UserExists { exists: Option<bool> }
    let exists = UserExists::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT EXISTS(SELECT 1 FROM user_account WHERE id = $1) AS exists",
        [other_user_id.into()],
    ))
    .one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !exists.map(|e| e.exists.unwrap_or(false)).unwrap_or(false) {
        return Err(StatusCode::NOT_FOUND);
    }

    // Find existing DM room between these two users
    let existing = DmRoomResult::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        r#"SELECT cr.id AS room_id
           FROM chat_room cr
           WHERE cr.kind = 'direct' AND cr.is_active = TRUE
             AND EXISTS(SELECT 1 FROM chat_room_member WHERE room_id = cr.id AND user_id = $1)
             AND EXISTS(SELECT 1 FROM chat_room_member WHERE room_id = cr.id AND user_id = $2)
             AND (SELECT COUNT(*) FROM chat_room_member WHERE room_id = cr.id) = 2
           LIMIT 1"#,
        [user.user_id.into(), other_user_id.into()],
    ))
    .one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(row) = existing {
        return Ok(Json(serde_json::json!({ "room_id": row.room_id })));
    }

    // Create new DM room
    #[derive(FromQueryResult)]
    struct IdRow { id: i32 }

    let room = IdRow::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "INSERT INTO chat_room (name, kind) VALUES ('DM', 'direct') RETURNING id",
        [],
    ))
    .one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Add both users as members
    state.db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "INSERT INTO chat_room_member (room_id, user_id) VALUES ($1, $2), ($1, $3)",
        [room.id.into(), user.user_id.into(), other_user_id.into()],
    ))
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({ "room_id": room.id })))
}

// ---------------------------------------------------------------------------
// Users available for DM
// ---------------------------------------------------------------------------

#[derive(Serialize, FromQueryResult)]
struct DmUser {
    id: i32,
    label: String,
    callsign: Option<String>,
    rank: Option<String>,
    org_label: Option<String>,
}

async fn list_dm_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<DmUser>>, StatusCode> {
    let user = require_auth(&state.db, &headers).await?;

    let users = DmUser::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        r#"SELECT
                ua.id,
                COALESCE(ua.callsign, ua.display_name, ua.login) AS label,
                ua.callsign,
                ua.rank,
                (SELECT o.short_name FROM user_role ur
                 JOIN org o ON o.id = ur.org_id
                 WHERE ur.user_id = ua.id
                 ORDER BY ur.id LIMIT 1
                ) AS org_label
            FROM user_account ua
            WHERE ua.is_active = TRUE AND ua.id <> $1
            ORDER BY label"#,
        [user.user_id.into()],
    ))
    .all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(users))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn chat_router() -> Router<AppState> {
    Router::new()
        .route("/api/chat/rooms", get(list_rooms))
        .route("/api/chat/rooms/:room_id/messages", get(room_messages))
        .route("/api/chat/rooms/:room_id/messages", post(send_message))
        .route("/api/chat/rooms/:room_id/stream", get(room_stream))
        .route("/api/chat/dm/:user_id", post(open_dm))
        .route("/api/chat/users", get(list_dm_users))
        .route("/api/chat/messages/:msg_id", axum::routing::put(edit_message))
        .route("/api/chat/messages/:msg_id", axum::routing::delete(delete_message))
        .route("/api/chat/media", post(upload_media))
        .route("/api/chat/media/:filename", get(serve_media))
}
