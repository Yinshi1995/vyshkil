//! Транзакційний outbox (09-messaging.md §3.1, `.claude/decisions/broker-nats-jetstream.md`) —
//! доменна дія пише сюди В ТІЙ САМІЙ транзакції, що свої дані; relay (фонова задача в `server`,
//! Фаза 1) публікує в NATS окремо, поза цією транзакцією. Прямий publish з обробника запиту
//! заборонено спекою — єдиний легітимний шлях повідомлення з домену назовні: рядок тут.
//!
//! `repo/` навмисно не знає про `contracts::Envelope<T>` (той самий принцип, що решта repo — SQL
//! по колонках, доменну форму збирає виклик): `payload_json`/`headers_json` приходять уже
//! серіалізованими рядками.

use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};
use uuid::Uuid;

/// Повертає `id` рядка — те саме значення стає `Nats-Msg-Id` при публікації (relay, Фаза 1):
/// повторна публікація після збою relay між "опубліковано" й "позначено published_at" безпечна,
/// бо брокер сам відкидає дублікат за цим ключем.
pub async fn insert(
    db: &impl ConnectionTrait,
    subject: &str,
    payload_json: &str,
    headers_json: &str,
) -> Result<Uuid, DbErr> {
    let id = Uuid::now_v7();
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO outbox (id, subject, payload, headers) \
         VALUES ($1::uuid, $2, $3::jsonb, $4::jsonb)",
        [id.to_string().into(), subject.into(), payload_json.into(), headers_json.into()],
    ))
    .await?;
    Ok(id)
}

/// Відставання (09 §5: "Адмін-екран «Черги»: відставання outbox — непубліковані, найстаріший").
#[derive(FromQueryResult)]
pub struct OutboxBacklog {
    pub unpublished_count: i64,
    pub oldest_unpublished_at: Option<String>,
}

pub async fn backlog(db: &DatabaseConnection) -> Result<OutboxBacklog, DbErr> {
    OutboxBacklog::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT COUNT(*) AS unpublished_count, \
                to_char(MIN(created_at), 'YYYY-MM-DD\"T\"HH24:MI:SS') AS oldest_unpublished_at \
         FROM outbox WHERE published_at IS NULL",
        [],
    ))
    .one(db)
    .await?
    .ok_or_else(|| DbErr::Custom("COUNT завжди повертає рядок".into()))
}
