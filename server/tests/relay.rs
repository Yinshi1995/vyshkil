//! Інтеграційний тест — не мок (09-messaging.md §6): `repo::outbox::insert` → `relay::relay_batch`
//! → РЕАЛЬНИЙ `nats-server -js`, і назад — тестовий підписник читає, що relay опублікував.
//! Потребує `TEST_DATABASE_URL` і `TEST_NATS_URL`; без будь-якого з двох — пропущено (той самий
//! підхід, що `app/tests/common::fresh_test_db`/`bus/tests/pubsub.rs`).

use app::backend::repo::outbox;
use async_nats::jetstream::consumer::AckPolicy;
use async_nats::jetstream::consumer::PullConsumer;
use async_nats::jetstream::stream::Config as StreamConfig;
use contracts::{subjects, DiscrepancyMetric, DiscrepancyOpened, Envelope};
use futures::StreamExt;
use migration::MigratorTrait;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, FromQueryResult};
use server::relay::relay_batch;
use std::time::Duration;
use uuid::Uuid;

/// Дзеркалить `app/tests/common::fresh_test_db` — окремий крейт, спільний тест-хелпер між
/// `app`/`server` не заводжу заради ~20 рядків (свідоме дублювання, не забуте).
async fn fresh_test_db() -> Option<DatabaseConnection> {
    let Ok(test_url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!(
            "TEST_DATABASE_URL не задано — інтеграційний тест server::relay пропущено \
             (див. .claude/memory/MEMORY.md)"
        );
        return None;
    };
    let slash = test_url.rfind('/').expect("TEST_DATABASE_URL має бути виду postgres://.../ім'я_бази");
    let db_name = &test_url[slash + 1..];
    assert!(
        !db_name.is_empty() && db_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "ім'я тестової бази має містити лише [a-zA-Z0-9_]: {db_name:?}"
    );
    let admin_url = format!("{}/postgres", &test_url[..slash]);

    let admin_db = Database::connect(&admin_url).await.expect("maintenance-база 'postgres'");
    admin_db
        .execute_unprepared(&format!(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity \
             WHERE datname = '{db_name}' AND pid <> pg_backend_pid()"
        ))
        .await
        .expect("розірвати старі з'єднання з тестовою базою");
    admin_db.execute_unprepared(&format!("DROP DATABASE IF EXISTS {db_name}")).await.expect("drop");
    admin_db.execute_unprepared(&format!("CREATE DATABASE {db_name}")).await.expect("create");

    let db = Database::connect(&test_url).await.expect("з'єднання з тестовою базою");
    migration::Migrator::up(&db, None).await.expect("міграції на тестовій базі");
    Some(db)
}

fn nats_url() -> Option<String> {
    match std::env::var("TEST_NATS_URL") {
        Ok(url) => Some(url),
        Err(_) => {
            eprintln!("TEST_NATS_URL не задано — інтеграційний тест server::relay пропущено");
            None
        }
    }
}

async fn outbox_row_count(db: &DatabaseConnection) -> i64 {
    #[derive(sea_orm::FromQueryResult)]
    struct Count {
        n: i64,
    }
    Count::find_by_statement(sea_orm::Statement::from_string(
        db.get_database_backend(),
        "SELECT COUNT(*) AS n FROM outbox",
    ))
    .one(db)
    .await
    .expect("порахувати outbox")
    .expect("COUNT завжди повертає рядок")
    .n
}

#[tokio::test]
async fn rolled_back_transaction_leaves_no_outbox_row() {
    let Some(db) = fresh_test_db().await else { return };
    use sea_orm::TransactionTrait;

    let txn = db.begin().await.expect("почати транзакцію");
    outbox::insert(&txn, subjects::DISCREPANCY_OPENED_V1, "{}", "{}").await.expect("insert в outbox");
    txn.rollback().await.expect("відкотити транзакцію");

    assert_eq!(outbox_row_count(&db).await, 0, "відкочена транзакція не мала лишити рядок в outbox");
}

#[tokio::test]
async fn relay_publishes_real_outbox_row_and_a_subscriber_receives_it() {
    let Some(db) = fresh_test_db().await else { return };
    let Some(url) = nats_url() else { return };

    let client = bus::connect(&url).await.expect("з'єднання з NATS");
    let js = bus::jetstream(&client);

    // Та сама назва й subject, що РЕАЛЬНИЙ `relay::run` створює (`contracts::subjects::
    // stream::EVENTS`, не окрема "TEST_EVENTS") -- навмисно: окрема тестова назва зі СПІЛЬНИМ
    // subject-патерном (`vyshkil.discrepancy.>`) одного разу вже призвела до "subjects overlap
    // with an existing stream" (JetStream забороняє двом РІЗНИМ стрімам ділити перекриті
    // subject-и) -- лишений тестовий стрім назавжди блокував створення реального. get-or-create
    // ідемпотентний, тому той самий виклик, що робить продакшен-relay, тут безпечний для
    // повторних прогонів тесту.
    let stream = js
        .get_or_create_stream(StreamConfig {
            name: subjects::stream::EVENTS.to_string(),
            subjects: vec!["vyshkil.discrepancy.>".to_string()],
            ..Default::default()
        })
        .await
        .expect("стрім для discrepancy-подій");

    let consumer: PullConsumer = stream
        .get_or_create_consumer(
            "test-relay-consumer",
            async_nats::jetstream::consumer::pull::Config {
                durable_name: Some("test-relay-consumer".to_string()),
                filter_subject: subjects::DISCREPANCY_OPENED_V1.to_string(),
                ack_policy: AckPolicy::Explicit,
                ack_wait: Duration::from_secs(5),
                ..Default::default()
            },
        )
        .await
        .expect("consumer");

    // Реальний envelope, той самий тип, що `repo::reconciliation` кладе в outbox — не вигаданий
    // payload заради тесту.
    let envelope = Envelope::new(
        "vyshkil.discrepancy.opened.v1",
        1,
        "server-relay-test",
        Uuid::now_v7(),
        None,
        DiscrepancyOpened {
            discrepancy_id: 999_999,
            org_id: 1,
            group_id: Some(1),
            metric: DiscrepancyMetric::ArrivedCount,
        },
    );
    let payload_json = serde_json::to_string(&envelope).expect("серіалізація");

    let id = outbox::insert(&db, subjects::DISCREPANCY_OPENED_V1, &payload_json, "{}")
        .await
        .expect("insert в outbox");

    relay_batch(&db, &js).await.expect("relay_batch");

    // published_at мусить бути проставлений.
    #[derive(sea_orm::FromQueryResult)]
    struct PublishedAt {
        published: bool,
    }
    let row = PublishedAt::find_by_statement(sea_orm::Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT published_at IS NOT NULL AS published FROM outbox WHERE id = $1::uuid",
        [id.to_string().into()],
    ))
    .one(&db)
    .await
    .expect("прочитати рядок outbox")
    .expect("рядок мав існувати");
    assert!(row.published, "relay мав позначити published_at після успішної публікації");

    // І реальний підписник (durable pull consumer, не той самий процес, що publish) реально
    // отримує байти, які поклав relay -- не мок, справжній NATS round-trip.
    let mut messages =
        consumer.fetch().max_messages(1).expires(Duration::from_secs(5)).messages().await.expect("fetch");
    let msg = messages.next().await.expect("повідомлення мало прийти").expect("без помилки");
    let received: Envelope<DiscrepancyOpened> =
        serde_json::from_slice(&msg.payload).expect("десеріалізація отриманого конверта");
    assert_eq!(received.payload.discrepancy_id, 999_999);
    msg.ack().await.expect("ack");

    // Стрім НЕ видаляємо (спільний з продакшен-relay, §вище) -- лише свій durable consumer,
    // щоб повторні прогони тесту не накопичували їх на реальному EVENTS-стрімі.
    let stream = js.get_stream(subjects::stream::EVENTS).await.expect("отримати стрім назад");
    stream.delete_consumer("test-relay-consumer").await.expect("прибрати consumer");
}

/// 09-messaging.md §6: "NATS вимкнено → outbox копиться → увімкнено → усе доставлено рівно раз".
/// Тут симулюємо не розрив самого з'єднання (те вже покриває `relay::run`'s ретрай-цикл,
/// окремий від `relay_batch`), а конкретний СПОСІБ, яким публікація реально провалюється в
/// проді: subject, для якого ще нема стріму ("no responders"/"no stream matches subject") --
/// той самий `relay_batch`, що й у "щасливому" тесті вище, викликається ДВІЧІ: спершу без
/// стріму (провал, рядок лишається неопублікованим, attempts++), потім зі стрімом (успіх).
#[tokio::test]
async fn outbox_row_survives_publish_failure_and_delivers_once_nats_catches_up() {
    let Some(db) = fresh_test_db().await else { return };
    let Some(url) = nats_url() else { return };

    let client = bus::connect(&url).await.expect("з'єднання з NATS");
    let js = bus::jetstream(&client);

    // Унікальний subject (НЕ реальний `vyshkil.discrepancy.>` -- той уже покритий спільним
    // EVENTS-стрімом з попередніх тестів/розробки, тому публікація туди завжди "випадково"
    // вдається; тут навмисно потрібен subject, для якого стріму ТОЧНО ще нема).
    let subject = format!("vyshkil.test-retry.{}.v1", Uuid::now_v7().simple());
    let payload_json = "{\"hello\":\"retry\"}".to_string();
    let id = outbox::insert(&db, &subject, &payload_json, "{}").await.expect("insert в outbox");

    // Спроба 1: стріму для цього subject-а ще нема -- публікація провалюється, рядок лишається
    // неопублікованим.
    relay_batch(&db, &js).await.expect("relay_batch (navіть при невдалій публікації не падає)");

    #[derive(sea_orm::FromQueryResult)]
    struct Row {
        published: bool,
        attempts: i32,
    }
    let after_failure = Row::find_by_statement(sea_orm::Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT published_at IS NOT NULL AS published, attempts FROM outbox WHERE id = $1::uuid",
        [id.to_string().into()],
    ))
    .one(&db)
    .await
    .expect("прочитати рядок")
    .expect("рядок мав існувати");
    assert!(!after_failure.published, "без стріму публікація мала провалитись, не позначитись");
    assert_eq!(after_failure.attempts, 1, "невдала спроба мала інкрементувати attempts");

    // "NATS/стрім стає доступним" -- створюємо стрім для цього subject-а (той самий idempotent
    // get-or-create виклик, що робить продакшен-код при відновленні з'єднання).
    js.get_or_create_stream(StreamConfig { name: format!("TEST_RETRY_{}", Uuid::now_v7().simple()), subjects: vec![subject.clone()], ..Default::default() })
        .await
        .expect("стрім для retry-subject-а");

    // Спроба 2: тепер публікація має вдатись.
    relay_batch(&db, &js).await.expect("relay_batch");

    let after_recovery = Row::find_by_statement(sea_orm::Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT published_at IS NOT NULL AS published, attempts FROM outbox WHERE id = $1::uuid",
        [id.to_string().into()],
    ))
    .one(&db)
    .await
    .expect("прочитати рядок")
    .expect("рядок мав існувати")
    .published;
    assert!(after_recovery, "після появи стріму повторна спроба мала опублікувати рядок");
}
