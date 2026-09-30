//! Ідемпотентне створення/отримання стріму (09-messaging.md §3.3 таблиця) — викликається на
//! старті процесу-власника стріму (relay в `server` для `EVENTS`; `notifier` для
//! `NOTIFY_CMD`/`NOTIFY_RESULT`), не за кожної публікації.

use async_nats::jetstream::stream::{Config as StreamConfig, Stream};

use crate::error::BusError;
use crate::BusResult;

pub async fn ensure_stream(
    js: &async_nats::jetstream::Context,
    name: &str,
    subjects: Vec<String>,
) -> BusResult<Stream> {
    js.get_or_create_stream(StreamConfig { name: name.to_string(), subjects, ..Default::default() })
        .await
        .map_err(|e| BusError(format!("створення/отримання стріму {name}: {e}")))
}
