use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::types::actor::Actor;
use crate::types::auth::PasskeyInfo;

// ---------------------------------------------------------------------------
// Спільний helper — require_admin
// ---------------------------------------------------------------------------

#[cfg(feature = "ssr")]
fn require_admin(actor: Option<Actor>) -> Result<Actor, ServerFnError> {
    use crate::backend::policy;
    let actor = actor.ok_or_else(|| ServerFnError::new("оберіть актора вгорі"))?;
    if !policy::is_admin(actor) {
        return Err(ServerFnError::new("лише адміністратор має доступ до цієї функції"));
    }
    Ok(actor)
}

// ---------------------------------------------------------------------------
// Learned-синоніми (ex pages/dictionaries/server.rs)
// ---------------------------------------------------------------------------

#[server(GetLearnedAliases, "/api")]
pub async fn get_learned_aliases(
    actor: Option<Actor>,
) -> Result<Vec<crate::types::dictionaries::LearnedAlias>, ServerFnError> {
    use crate::backend::repo;
    require_admin(actor)?;
    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::dictionaries::learned_aliases(&db)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(ConfirmLearnedAlias, "/api")]
pub async fn confirm_learned_alias(actor: Option<Actor>, alias_id: i32) -> Result<(), ServerFnError> {
    use crate::backend::{db, repo};
    require_admin(actor)?;
    let txn = db::actor_transaction(actor.unwrap()).await.map_err(|e| ServerFnError::new(e.to_string()))?;
    repo::dictionaries::confirm_learned_alias(&txn, alias_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    txn.commit().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server(RejectLearnedAlias, "/api")]
pub async fn reject_learned_alias(actor: Option<Actor>, alias_id: i32) -> Result<(), ServerFnError> {
    use crate::backend::{db, repo};
    require_admin(actor)?;
    let txn = db::actor_transaction(actor.unwrap()).await.map_err(|e| ServerFnError::new(e.to_string()))?;
    repo::dictionaries::reject_learned_alias(&txn, alias_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    txn.commit().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// WhatsApp (ex pages/admin_whatsapp/server.rs)
// ---------------------------------------------------------------------------

const SUBJECT_PAIR_CODE: &str = "vyshkil.notifier.whatsapp.pair.code";
const SUBJECT_LOGOUT: &str = "vyshkil.notifier.whatsapp.logout";
const SUBJECT_TEST: &str = "vyshkil.notifier.whatsapp.test";

#[cfg(feature = "ssr")]
async fn nats_request(subject: &str, payload_json: String) -> Result<String, ServerFnError> {
    let shared = expect_context::<bus::SharedNatsClient>();
    let client = { shared.lock().map_err(|_| ServerFnError::new("NATS mutex отруєний"))?.clone() };
    let client = client.ok_or_else(|| ServerFnError::new("NATS недоступний"))?;
    let reply = bus::request(&client, subject, payload_json.into_bytes())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    String::from_utf8(reply).map_err(|e| ServerFnError::new(format!("невалідна відповідь: {e}")))
}

#[derive(Deserialize)]
struct ErrorReply {
    error: Option<String>,
}

fn check_error(raw: &str) -> Result<(), ServerFnError> {
    if let Ok(ErrorReply { error: Some(e) }) = serde_json::from_str::<ErrorReply>(raw) {
        return Err(ServerFnError::new(e));
    }
    Ok(())
}

#[derive(Deserialize)]
struct PairCodeReply {
    pairing_code: Option<String>,
}

#[server(RequestPairingCode, "/api")]
pub async fn request_pairing_code(actor: Option<Actor>, phone: String) -> Result<String, ServerFnError> {
    require_admin(actor)?;
    let payload = serde_json::json!({ "phone": phone }).to_string();
    let raw = nats_request(SUBJECT_PAIR_CODE, payload).await?;
    check_error(&raw)?;
    let reply: PairCodeReply =
        serde_json::from_str(&raw).map_err(|e| ServerFnError::new(format!("невалідна відповідь: {e}")))?;
    reply.pairing_code.ok_or_else(|| ServerFnError::new("notifier не повернув pairing-код"))
}

#[server(LogoutWhatsapp, "/api")]
pub async fn logout_whatsapp(actor: Option<Actor>) -> Result<(), ServerFnError> {
    require_admin(actor)?;
    let raw = nats_request(SUBJECT_LOGOUT, "{}".to_string()).await?;
    check_error(&raw)
}

#[server(SendTestNotification, "/api")]
pub async fn send_test_notification(actor: Option<Actor>, org_id: i32) -> Result<String, ServerFnError> {
    require_admin(actor)?;
    let payload = serde_json::json!({ "org_id": org_id }).to_string();
    let raw = nats_request(SUBJECT_TEST, payload).await?;
    check_error(&raw)?;
    Ok(raw)
}

// ---------------------------------------------------------------------------
// Черги (ex pages/admin_queues/server.rs)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxBacklogDto {
    pub unpublished_count: i64,
    pub oldest_unpublished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStatusDto {
    pub name: String,
    pub messages: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlqEntryDto {
    pub seq: u64,
    pub original_subject: String,
    pub reason: String,
    pub failed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatusDto {
    pub outbox: OutboxBacklogDto,
    pub streams: Vec<StreamStatusDto>,
    pub dlq: Vec<DlqEntryDto>,
    pub nats_connected: bool,
}

const KNOWN_STREAMS: [&str; 4] = ["EVENTS", "NOTIFY_CMD", "NOTIFY_RESULT", "DLQ"];
const DLQ_STREAM: &str = "DLQ";
const DLQ_DISPLAY_LIMIT: u64 = 20;

#[cfg(feature = "ssr")]
#[derive(Deserialize)]
struct RawDlqEntry {
    original_subject: String,
    reason: String,
    payload_base64: String,
    failed_at: String,
}

#[server(GetQueueStatus, "/api")]
pub async fn get_queue_status(actor: Option<Actor>) -> Result<QueueStatusDto, ServerFnError> {
    require_admin(actor)?;

    let db = expect_context::<sea_orm::DatabaseConnection>();
    let backlog = crate::backend::repo::outbox::backlog(&db)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let outbox =
        OutboxBacklogDto { unpublished_count: backlog.unpublished_count, oldest_unpublished_at: backlog.oldest_unpublished_at };

    let shared = expect_context::<bus::SharedNatsClient>();
    let client = { shared.lock().map_err(|_| ServerFnError::new("NATS mutex отруєний"))?.clone() };
    let Some(client) = client else {
        return Ok(QueueStatusDto { outbox, streams: vec![], dlq: vec![], nats_connected: false });
    };
    let js = bus::jetstream(&client);

    let mut streams = Vec::with_capacity(KNOWN_STREAMS.len());
    for name in KNOWN_STREAMS {
        let messages = match js.get_stream(name).await {
            Ok(mut s) => s.info().await.ok().map(|i| i.state.messages),
            Err(_) => None,
        };
        streams.push(StreamStatusDto { name: name.to_string(), messages });
    }

    let mut dlq = Vec::new();
    if let Ok(mut stream) = js.get_stream(DLQ_STREAM).await {
        if let Ok(info) = stream.info().await {
            let last = info.state.last_sequence;
            let first = last.saturating_sub(DLQ_DISPLAY_LIMIT).max(info.state.first_sequence).max(1);
            for seq in (first..=last).rev() {
                let Ok(raw) = stream.get_raw_message(seq).await else { continue };
                let Ok(entry) = serde_json::from_slice::<RawDlqEntry>(&raw.payload) else { continue };
                dlq.push(DlqEntryDto {
                    seq,
                    original_subject: entry.original_subject,
                    reason: entry.reason,
                    failed_at: entry.failed_at,
                });
            }
        }
    }

    Ok(QueueStatusDto { outbox, streams, dlq, nats_connected: true })
}

#[server(RetryDlqEntry, "/api")]
pub async fn retry_dlq_entry(actor: Option<Actor>, seq: u64) -> Result<(), ServerFnError> {
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;

    require_admin(actor)?;

    let shared = expect_context::<bus::SharedNatsClient>();
    let client = { shared.lock().map_err(|_| ServerFnError::new("NATS mutex отруєний"))?.clone() };
    let client = client.ok_or_else(|| ServerFnError::new("NATS недоступний"))?;
    let js = bus::jetstream(&client);

    let stream =
        js.get_stream(DLQ_STREAM).await.map_err(|e| ServerFnError::new(format!("DLQ стрім: {e}")))?;
    let raw = stream
        .get_raw_message(seq)
        .await
        .map_err(|e| ServerFnError::new(format!("читання запису {seq}: {e}")))?;
    let entry: RawDlqEntry = serde_json::from_slice(&raw.payload)
        .map_err(|e| ServerFnError::new(format!("розбір запису {seq}: {e}")))?;
    let payload = BASE64
        .decode(&entry.payload_base64)
        .map_err(|e| ServerFnError::new(format!("декодування payload: {e}")))?;

    let new_msg_id = uuid::Uuid::now_v7().to_string();
    bus::publish_raw_with_msg_id(&js, &entry.original_subject, &new_msg_id, payload)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    stream
        .delete_message(seq)
        .await
        .map_err(|e| ServerFnError::new(format!("видалення запису {seq}: {e}")))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Passkey / FIDO2 (12-auth.md §1.3, Étap 10b)
// ---------------------------------------------------------------------------

#[server(ListPasskeys, "/api")]
pub async fn list_passkeys() -> Result<Vec<PasskeyInfo>, ServerFnError> {
    use crate::backend::repo;
    use crate::services::auth::require_auth;

    let auth_user = require_auth().await?;
    let db = expect_context::<sea_orm::DatabaseConnection>();
    let creds = repo::passkeys::credentials_for_user(&db, auth_user.user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(creds
        .into_iter()
        .map(|c| PasskeyInfo {
            id: c.id,
            name: c.name,
            created_at: String::new(),
        })
        .collect())
}

#[server(DeletePasskey, "/api")]
pub async fn delete_passkey(credential_id: i32) -> Result<bool, ServerFnError> {
    use crate::backend::repo;
    use crate::services::auth::require_auth;

    let auth_user = require_auth().await?;
    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::passkeys::delete_credential(&db, auth_user.user_id, credential_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
