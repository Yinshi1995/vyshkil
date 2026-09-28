//! SQL для агрегату "групи на навчанні" (01 §3) — лише читання подій, потрібне для воронки.
//! Сама арифметика — в `domain::counting` (чиста, без БД); тут лише вибірка й перетворення типів.

use crate::domain::counting::{EventType, GroupEventRecord};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};

/// Усі події групи, найстаріша перша — досить для будь-якої функції `domain::counting`
/// (`in_training`/`events_on`/`finishing_on` самі фільтрують за датою/`known_at`).
pub async fn group_events(
    db: &DatabaseConnection,
    group_id: i32,
) -> Result<Vec<GroupEventRecord>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        event_type: String,
        count: i32,
        occurred_on: String,
        recorded_at: String,
    }

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"
        SELECT
            event_type,
            count,
            to_char(occurred_on, 'YYYY-MM-DD') AS occurred_on,
            to_char(recorded_at, 'YYYY-MM-DD"T"HH24:MI:SS') AS recorded_at
        FROM group_event
        WHERE group_id = $1
        ORDER BY occurred_on, id
        "#,
        [group_id.into()],
    );

    let rows = Row::find_by_statement(stmt).all(db).await?;

    Ok(rows
        .into_iter()
        .filter_map(|r| {
            EventType::parse(&r.event_type).map(|event_type| GroupEventRecord {
                event_type,
                count: r.count,
                occurred_on: r.occurred_on,
                recorded_at: r.recorded_at,
            })
        })
        .collect())
}
