//! Relay: фонова задача, що читає непубліковані рядки `outbox` і публікує їх у NATS JetStream
//! (09-messaging.md §3.1). Таймер-опитувач, не `LISTEN/NOTIFY` — рішення й чому:
//! `migration::m20260930_000042_create_outbox_table`, `.claude/decisions/
//! broker-nats-jetstream.md`. `SELECT ... FOR UPDATE SKIP LOCKED` кожні `POLL_INTERVAL`, батч
//! публікується через `bus::publish_raw_with_msg_id` з `outbox.id` як `Nats-Msg-Id`
//! (дедуплікація на боці брокера, якщо повторний тик публікує той самий рядок після збою між
//! "опубліковано" й "позначено published_at").
//!
//! NATS недоступний → застосунок далі працює нормально (§3.1: "NATS недоступний -> застосунок
//! працює як завжди, outbox накопичується, потім доганяє") — relay сам перепідключається на
//! наступному тику, нічого не панікує і не блокує запуск сервера.

use std::time::Duration;

use bus::{async_nats, SharedNatsClient};
use contracts::subjects;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement, TransactionTrait};

const POLL_INTERVAL: Duration = Duration::from_secs(2);
const BATCH_SIZE: u64 = 50;

/// Запускається одним `tokio::spawn` у `main.rs` (перша фонова задача в цьому проєкті) — сам
/// цикл ніколи не повертається, живе стільки ж, скільки процес сервера.
pub async fn run(db: DatabaseConnection, nats_url: String, shared: SharedNatsClient) {
    let mut client: Option<async_nats::Client> = None;

    loop {
        tokio::time::sleep(POLL_INTERVAL).await;

        if client.is_none() {
            match bus::connect(&nats_url).await {
                Ok(c) => {
                    let js = bus::jetstream(&c);
                    // Ідемпотентно (get-or-create) -- relay володіє схемою `EVENTS` (09 §3.3:
                    // "app" публікує доменні події), notifier аналогічно забезпечує СВОЇ стріми
                    // (`NOTIFY_CMD`/`NOTIFY_RESULT`) на власному старті.
                    // Вузько (не "vyshkil.>"): JetStream забороняє ДВОМ стрімам ділити
                    // перекриті subject-и, а `NOTIFY_CMD`/`NOTIFY_RESULT` (окремі стріми,
                    // §3.3-таблиця) теж під префіксом `vyshkil.*` -- широкий wildcard тут зробив
                    // би їх недостворюваними пізніше.
                    if let Err(e) = bus::ensure_stream(
                        &js,
                        subjects::stream::EVENTS,
                        vec!["vyshkil.discrepancy.>".to_string()],
                    )
                    .await
                    {
                        tracing::warn!("relay: не вдалось забезпечити стрім EVENTS: {e}");
                        continue;
                    }
                    tracing::info!("relay: з'єднано з NATS ({nats_url}), стрім EVENTS готовий");
                    *shared.lock().expect("shared NATS mutex отруєний") = Some(c.clone());
                    client = Some(c);
                }
                Err(e) => {
                    tracing::warn!("relay: не вдалось з'єднатися з NATS ({nats_url}): {e}");
                    continue;
                }
            }
        }
        // `async_nats::Client` сам перепідключається під капотом на тимчасову недоступність
        // (це його вбудована поведінка, не додаткова логіка тут) -- клієнта скидаємо в `None`
        // лише коли САМЕ початкове з'єднання (вище) не вдалось, ніколи через невдалу публікацію.
        let js = bus::jetstream(client.as_ref().expect("щойно перевірено is_none() вище"));

        if let Err(e) = relay_batch(&db, &js).await {
            // `relay_batch` повертає `Err` лише з боку БД (транзакція/SELECT/UPDATE) -- помилки
            // самої публікації воно ловить порядково всередині й пише в attempts/last_error,
            // не пропускає сюди.
            tracing::warn!("relay: помилка читання/оновлення outbox: {e}");
        }
    }
}

// `id: String` (cast `::text` у запиті), не `uuid::Uuid` -- sea-orm без фічі "with-uuid" (той
// самий принцип, що `repo::reconciliation`: не тягнути зайву фічу заради одного стовпця, дати
// теж читаються `to_char(...)`-рядками).
#[derive(FromQueryResult)]
struct OutboxRow {
    id: String,
    subject: String,
    payload: String,
}

/// `pub` лише заради `tests/relay.rs` (09 §6: інтеграційний тест проти реального `nats-server`)
/// — викликачі поза тестами йдуть через `run()`, не цю функцію напряму.
pub async fn relay_batch(db: &DatabaseConnection, js: &async_nats::jetstream::Context) -> Result<(), DbErr> {
    let txn = db.begin().await?;

    let rows = OutboxRow::find_by_statement(Statement::from_sql_and_values(
        txn.get_database_backend(),
        "SELECT id::text AS id, subject, payload::text AS payload FROM outbox \
         WHERE published_at IS NULL ORDER BY id LIMIT $1 FOR UPDATE SKIP LOCKED",
        [BATCH_SIZE.into()],
    ))
    .all(&txn)
    .await?;

    for row in rows {
        match bus::publish_raw_with_msg_id(js, &row.subject, &row.id, row.payload.into_bytes()).await {
            Ok(()) => {
                txn.execute(Statement::from_sql_and_values(
                    txn.get_database_backend(),
                    "UPDATE outbox SET published_at = now() WHERE id = $1::uuid",
                    [row.id.into()],
                ))
                .await?;
            }
            Err(e) => {
                tracing::warn!("relay: публікація {} ({}) не вдалась: {e}", row.id, row.subject);
                txn.execute(Statement::from_sql_and_values(
                    txn.get_database_backend(),
                    "UPDATE outbox SET attempts = attempts + 1, last_error = $2 WHERE id = $1::uuid",
                    [row.id.into(), e.to_string().into()],
                ))
                .await?;
                // Одна невдача (NATS щойно відпав) не має ламати весь батч -- решта рядків теж
                // спробують, наступний тик підбере й цей знову (published_at лишається NULL).
            }
        }
    }

    txn.commit().await
}
