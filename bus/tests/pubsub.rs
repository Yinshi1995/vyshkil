//! Інтеграційний тест проти РЕАЛЬНОГО `nats-server -js` (09-messaging.md §6) — не мок. Потребує
//! `TEST_NATS_URL` (напр. `nats://127.0.0.1:4222`); без нього — пропущено (той самий підхід, що
//! `app/tests/common::fresh_test_db` для `TEST_DATABASE_URL`).

use std::time::Duration;

use async_nats::jetstream::stream::Config as StreamConfig;
use bus::{connect, ensure_pull_consumer, jetstream, publish_with_msg_id, ConsumerSpec};
use contracts::Envelope;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TestPayload {
    n: i32,
}

async fn nats_url() -> Option<String> {
    match std::env::var("TEST_NATS_URL") {
        Ok(url) => Some(url),
        Err(_) => {
            eprintln!(
                "TEST_NATS_URL не задано — інтеграційний тест bus пропущено \
                 (напр. nats://127.0.0.1:4222, `docker run -d -p 4222:4222 nats:2-alpine -js`)"
            );
            None
        }
    }
}

#[tokio::test]
async fn publish_then_pull_consume_roundtrips_payload() {
    let Some(url) = nats_url().await else { return };
    let client = connect(&url).await.expect("з'єднання з NATS");
    let js = jetstream(&client);

    // Унікальна назва стріму на прогін тесту -- паралельні прогони/повторні запуски не колізять.
    let stream_name = format!("TEST_BUS_{}", Uuid::now_v7().simple());
    let subject = format!("test.bus.{}", Uuid::now_v7().simple());

    let stream = js
        .create_stream(StreamConfig {
            name: stream_name.clone(),
            subjects: vec![subject.clone()],
            ..Default::default()
        })
        .await
        .expect("створення тестового стріму");

    let envelope = Envelope::new(
        "test.event.v1",
        1,
        "bus-test",
        Uuid::now_v7(),
        None,
        TestPayload { n: 7 },
    );
    let msg_id = Uuid::now_v7().to_string();

    publish_with_msg_id(&js, &subject, &msg_id, &envelope).await.expect("публікація");

    let consumer = ensure_pull_consumer(
        &stream,
        ConsumerSpec {
            durable_name: "test-consumer".to_string(),
            filter_subject: subject.clone(),
            ack_wait: Duration::from_secs(5),
            max_deliver: 3,
        },
    )
    .await
    .expect("consumer");

    let mut messages =
        consumer.fetch().max_messages(1).expires(Duration::from_secs(5)).messages().await.expect("fetch");
    let msg = messages.next().await.expect("повідомлення мало прийти").expect("повідомлення без помилки");
    let received: Envelope<TestPayload> = serde_json::from_slice(&msg.payload).expect("десеріалізація");
    assert_eq!(received.payload, TestPayload { n: 7 });
    msg.ack().await.expect("ack");

    js.delete_stream(&stream_name).await.expect("прибрати тестовий стрім");
}

#[tokio::test]
async fn duplicate_msg_id_is_deduplicated_by_broker() {
    let Some(url) = nats_url().await else { return };
    let client = connect(&url).await.expect("з'єднання з NATS");
    let js = jetstream(&client);

    let stream_name = format!("TEST_DEDUP_{}", Uuid::now_v7().simple());
    let subject = format!("test.dedup.{}", Uuid::now_v7().simple());

    js.create_stream(StreamConfig {
        name: stream_name.clone(),
        subjects: vec![subject.clone()],
        duplicate_window: Duration::from_secs(60),
        ..Default::default()
    })
    .await
    .expect("створення тестового стріму");

    let envelope = Envelope::new(
        "test.event.v1",
        1,
        "bus-test",
        Uuid::now_v7(),
        None,
        TestPayload { n: 1 },
    );
    let msg_id = Uuid::now_v7().to_string();

    // Публікуємо ДВІЧІ той самий msg_id -- relay саме так поводиться при повторі після збою
    // "опубліковано, але не встигли позначити published_at" (§3.1).
    publish_with_msg_id(&js, &subject, &msg_id, &envelope).await.expect("перша публікація");
    publish_with_msg_id(&js, &subject, &msg_id, &envelope).await.expect("друга публікація (дублікат)");

    let mut retrieved_stream = js.get_stream(&stream_name).await.expect("інфо про стрім");
    let info = retrieved_stream.info().await.expect("info");
    assert_eq!(info.state.messages, 1, "дублікат мав бути відкинутий брокером за Nats-Msg-Id");

    js.delete_stream(&stream_name).await.expect("прибрати тестовий стрім");
}
