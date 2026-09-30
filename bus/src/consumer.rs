//! Durable pull-consumer (09-messaging.md §3.7: "durable, ack_wait, max_deliver → DLQ, backoff").
//! Стрім мусить уже існувати (створюється окремо адмінським кроком/скриптом, не тут — `bus` не
//! вирішує, хто власник схеми стрімів).

use std::time::Duration;

use async_nats::jetstream::consumer::{pull::Config as PullConfig, AckPolicy};
use async_nats::jetstream::stream::Stream;
use async_nats::jetstream::consumer::PullConsumer;

use crate::error::BusError;
use crate::BusResult;

pub struct ConsumerSpec {
    pub durable_name: String,
    pub filter_subject: String,
    pub ack_wait: Duration,
    pub max_deliver: i64,
}

/// Створює consumer, якщо його ще нема (durable — та сама назва повертає той самий, ідемпотентно).
pub async fn ensure_pull_consumer(
    stream: &Stream,
    spec: ConsumerSpec,
) -> BusResult<PullConsumer> {
    stream
        .get_or_create_consumer(
            &spec.durable_name.clone(),
            PullConfig {
                durable_name: Some(spec.durable_name),
                filter_subject: spec.filter_subject,
                ack_policy: AckPolicy::Explicit,
                ack_wait: spec.ack_wait,
                max_deliver: spec.max_deliver,
                ..Default::default()
            },
        )
        .await
        .map_err(|e| BusError(format!("створення/отримання consumer'а: {e}")))
}
