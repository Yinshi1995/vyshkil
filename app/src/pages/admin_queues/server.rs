//! Адмін-екран "Черги" (09-messaging.md §5, Фаза 4) — відставання outbox, стан стрімів, DLQ з
//! повторною відправкою. Лише `admin`.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::types::actor::Actor;

// `#[cfg(feature = "ssr")]` явно — плоска допоміжна функція, не сама `#[server(...)]`-функція
// (той самий принцип, що `admin_whatsapp/server.rs::require_admin` — макрос сам вирізає тіло
// #[server] fn під non-ssr, звичайні helper-функції так не вміють).
#[cfg(feature = "ssr")]
fn require_admin(actor: Option<Actor>) -> Result<Actor, ServerFnError> {
    use crate::backend::policy;
    let actor = actor.ok_or_else(|| ServerFnError::new("оберіть актора вгорі"))?;
    if !policy::is_admin(actor) {
        return Err(ServerFnError::new("лише адміністратор бачить черги"));
    }
    Ok(actor)
}

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

/// Повторна відправка DLQ-запису (09 §5: "DLQ з кнопкою «повторити»") — декодує збережений
/// payload, публікує назад у `original_subject` з НОВИМ `Nats-Msg-Id` (не той самий, що оригінал
/// — інакше брокер міг би відкинути як дублікат), видаляє запис із DLQ, щойно republish вдався.
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
