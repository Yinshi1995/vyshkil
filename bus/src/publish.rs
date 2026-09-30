//! Публікація в JetStream з дедуплікаційним заголовком (09-messaging.md §3.1): `Nats-Msg-Id`
//! дозволяє брокеру самому відкидати повторну публікацію того самого `outbox.id` (relay може
//! впасти між "опубліковано" й "позначено published_at" — повтор безпечний).

use async_nats::HeaderMap;
use contracts::Envelope;
use serde::Serialize;

use crate::error::{BusError, BusResult};

pub async fn connect(url: &str) -> BusResult<async_nats::Client> {
    async_nats::connect(url).await.map_err(BusError::from)
}

/// Core NATS request-reply (не JetStream — разові команди адмінки, 09 §4), таймаут — вбудований
/// дефолт `async_nats::Client::request` (кілька секунд).
pub async fn request(client: &async_nats::Client, subject: &str, payload: Vec<u8>) -> BusResult<Vec<u8>> {
    let msg = client
        .request(subject.to_string(), payload.into())
        .await
        .map_err(|e| BusError(format!("request до {subject}: {e}")))?;
    Ok(msg.payload.to_vec())
}

pub fn jetstream(client: &async_nats::Client) -> async_nats::jetstream::Context {
    async_nats::jetstream::new(client.clone())
}

/// `msg_id` — ключ дедуплікації на боці NATS (типово `outbox.id.to_string()`, не `envelope.id`:
/// той генерується заново на кожну спробу публікації, тоді як рядок outbox — один назавжди).
pub async fn publish_with_msg_id<T: Serialize>(
    js: &async_nats::jetstream::Context,
    subject: &str,
    msg_id: &str,
    envelope: &Envelope<T>,
) -> BusResult<()> {
    let payload = serde_json::to_vec(envelope)?;
    publish_raw_with_msg_id(js, subject, msg_id, payload).await
}

/// Той самий publish, без ре-серіалізації: relay (`server`, Фаза 1) вже має готові байти з
/// `outbox.payload` (сам `repo::outbox::insert` серіалізує `Envelope` один раз, при записі в
/// транзакції домену) — публікує їх як є, не десеріалізує назад лише щоб серіалізувати заново.
pub async fn publish_raw_with_msg_id(
    js: &async_nats::jetstream::Context,
    subject: &str,
    msg_id: &str,
    payload: Vec<u8>,
) -> BusResult<()> {
    let mut headers = HeaderMap::new();
    headers.insert("Nats-Msg-Id", msg_id);
    let ack_future = js
        .publish_with_headers(subject.to_string(), headers, payload.into())
        .await
        .map_err(|e| BusError(format!("публікація в {subject}: {e}")))?;
    ack_future.await.map_err(|e| BusError(format!("очікування ack від {subject}: {e}")))?;
    Ok(())
}
