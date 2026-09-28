//! Єдиний модуль перевірки прав (docs/spec/01-domain-model.md §6). Кожна server function,
//! яка читає/пише доменні дані, викликає щось звідси першою — жодних перевірок прав в іншому місці.
//! Тип Actor/Role — в `app::actor` (спільний з UI-перемикачем), тут — лише логіка перевірки.

pub use app::types::actor::{Actor, Role};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, Statement};

/// Рядок для `SET LOCAL app.actor` — читає generic-тригер аудиту (audit_log.actor).
/// Не містить ПІБ: лише org_id (число) і роль.
pub fn session_tag(actor: Actor) -> String {
    format!("{}:{}", actor.org_id, actor.role.as_str())
}

/// admin редагує все; решта — тільки власні подання своєї організації (01 §6).
pub fn can_edit_org(actor: Actor, target_org_id: i32) -> bool {
    match actor.role {
        Role::Admin => true,
        Role::OrgEditor => actor.org_id == target_org_id,
        Role::Viewer => false,
    }
}

/// Перевіряє, чи бачить актор дані організації `target_org_id`: своя організація або її піддерево
/// за `subordination_closure` (обидві осі, на сьогодні) — 01 §6. `admin` бачить усе.
pub async fn can_view_org(
    db: &DatabaseConnection,
    actor: Actor,
    target_org_id: i32,
) -> Result<bool, DbErr> {
    if actor.role == Role::Admin {
        return Ok(true);
    }
    if actor.org_id == target_org_id {
        return Ok(true);
    }

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT EXISTS ( \
            SELECT 1 FROM subordination_closure \
            WHERE ancestor_id = $1 AND descendant_id = $2 \
              AND daterange(valid_from, valid_to, '[)') @> CURRENT_DATE \
        ) AS exists_flag",
        [actor.org_id.into(), target_org_id.into()],
    );

    let row = db.query_one(stmt).await?;
    match row {
        Some(row) => row.try_get::<bool>("", "exists_flag").or(Ok(false)),
        None => Ok(false),
    }
}

/// Виконує `SET LOCAL app.actor = '<org_id>:<role>'` на поточному з'єднанні/транзакції —
/// щоб generic-тригер аудиту (`audit_log_trigger`) знав актора без передачі його в кожен запит.
pub async fn set_session_actor(db: &impl ConnectionTrait, actor: Actor) -> Result<(), DbErr> {
    let sql = format!("SET LOCAL app.actor = '{}'", session_tag(actor));
    db.execute_unprepared(&sql).await?;
    Ok(())
}
