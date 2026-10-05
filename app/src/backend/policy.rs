//! Єдиний модуль перевірки прав (docs/spec/01-domain-model.md §6). Кожна server function,
//! яка читає/пише доменні дані, викликає щось звідси першою — жодних перевірок прав в іншому місці.
//! Тип Actor/Role — в `crate::types::actor` (спільний з UI-перемикачем), тут — лише логіка перевірки.

pub use crate::types::actor::{Actor, Role};
use crate::types::org::OrgTreeRow;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, Statement};

/// Рядок для `SET LOCAL app.actor` — читає generic-тригер аудиту (audit_log.actor).
/// Не містить ПІБ: лише org_id (число) і роль.
pub fn session_tag(actor: Actor) -> String {
    format!("{}:{}", actor.org_id, actor.role.as_str())
}

/// Дії, не прив'язані до конкретної організації (підтвердження learned-синонімів, керування
/// довідниками — 01 §"Навчання") — лише `admin`.
pub fn is_admin(actor: Actor) -> bool {
    actor.role == Role::Admin
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

/// Чи може адміністратор керувати (створювати/редагувати/видаляти) підрозділом `target_org_id`:
/// лише свій або підлеглий. На відміну від `can_view_org`, перевіряє підпорядкування навіть для
/// `Admin` — адмін керує тільки власним піддеревом (01 §6).
pub async fn can_manage_org(
    db: &DatabaseConnection,
    actor: Actor,
    target_org_id: i32,
) -> Result<bool, DbErr> {
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

/// Список org_id, якими актор може керувати: своя організація + піддерево за
/// `subordination_closure` (обидві осі, на сьогодні). На відміну від `visible_org_ids`, перевіряє
/// підпорядкування навіть для `admin` — адмін керує тільки власним піддеревом.
pub async fn manageable_org_ids(
    db: &DatabaseConnection,
    actor: Actor,
) -> Result<Vec<i32>, DbErr> {
    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT $1::int AS org_id \
         UNION \
         SELECT descendant_id AS org_id FROM subordination_closure \
         WHERE ancestor_id = $1 AND daterange(valid_from, valid_to, '[)') @> CURRENT_DATE",
        [actor.org_id.into()],
    );

    let rows = db.query_all(stmt).await?;
    rows.into_iter()
        .map(|r| r.try_get::<i32>("", "org_id"))
        .collect::<Result<Vec<_>, _>>()
}

/// Список org_id, видимих актору: сама організація + все піддерево за `subordination_closure`
/// (обидві осі, на сьогодні) — 01 §6. `None` = без обмежень (`admin` бачить усе); `Some(ids)` —
/// список для фільтрації результатів багаторядкових запитів (пошук, дерево), де перевіряти
/// кожен рядок окремим `can_view_org` було б зайвим SQL-запитом на рядок.
pub async fn visible_org_ids(
    db: &DatabaseConnection,
    actor: Actor,
) -> Result<Option<Vec<i32>>, DbErr> {
    if actor.role == Role::Admin {
        return Ok(None);
    }

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT $1::int AS org_id \
         UNION \
         SELECT descendant_id AS org_id FROM subordination_closure \
         WHERE ancestor_id = $1 AND daterange(valid_from, valid_to, '[)') @> CURRENT_DATE",
        [actor.org_id.into()],
    );

    let rows = db.query_all(stmt).await?;
    let ids = rows
        .into_iter()
        .map(|r| r.try_get::<i32>("", "org_id"))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(ids))
}

/// Обрізає дерево підпорядкування до видимих акторові вузлів (`visible_org_ids`). Вузол, чий
/// батько лишився поза видимою множиною (типово — власна організація актора: її "справжній"
/// батько актору не показуємо), стає для рендеру коренем (`parent_id = None`) — інакше клієнтський
/// рендер дерева (який будує його від `parent_id = None`) мовчки загубив би ціле піддерево.
pub fn restrict_tree(rows: Vec<OrgTreeRow>, visible: &[i32]) -> Vec<OrgTreeRow> {
    rows.into_iter()
        .filter(|r| visible.contains(&r.id))
        .map(|mut r| {
            if let Some(parent_id) = r.parent_id {
                if !visible.contains(&parent_id) {
                    r.parent_id = None;
                }
            }
            r
        })
        .collect()
}

/// Виконує `SET LOCAL app.actor = '<org_id>:<role>'` на поточному з'єднанні/транзакції —
/// щоб generic-тригер аудиту (`audit_log_trigger`) знав актора без передачі його в кожен запит.
pub async fn set_session_actor(db: &impl ConnectionTrait, actor: Actor) -> Result<(), DbErr> {
    let sql = format!("SET LOCAL app.actor = '{}'", session_tag(actor));
    db.execute_unprepared(&sql).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(org_id: i32, role: Role) -> Actor {
        Actor { org_id, role }
    }

    #[test]
    fn viewer_never_edits() {
        assert!(!can_edit_org(actor(1, Role::Viewer), 1), "viewer не редагує навіть свою організацію");
        assert!(!can_edit_org(actor(1, Role::Viewer), 2), "viewer не редагує чужу організацію");
    }

    #[test]
    fn org_editor_edits_only_own_org() {
        assert!(can_edit_org(actor(1, Role::OrgEditor), 1), "org_editor редагує свою організацію");
        assert!(!can_edit_org(actor(1, Role::OrgEditor), 2), "org_editor НЕ редагує чужу організацію");
    }

    #[test]
    fn only_admin_passes_is_admin() {
        assert!(is_admin(actor(1, Role::Admin)));
        assert!(!is_admin(actor(1, Role::OrgEditor)));
        assert!(!is_admin(actor(1, Role::Viewer)));
    }

    #[test]
    fn admin_edits_everything() {
        assert!(can_edit_org(actor(1, Role::Admin), 1));
        assert!(can_edit_org(actor(1, Role::Admin), 999));
    }

    #[test]
    fn restrict_tree_reroots_own_org_and_drops_invisible() {
        let rows = vec![
            OrgTreeRow { id: 1, label: "УВ(с)".into(), full_name: None, parent_id: None },
            OrgTreeRow { id: 2, label: "17 АК".into(), full_name: None, parent_id: Some(1) },
            OrgTreeRow { id: 3, label: "241 обр ТрО".into(), full_name: None, parent_id: Some(2) },
            OrgTreeRow { id: 4, label: "20 АК".into(), full_name: None, parent_id: Some(1) },
            OrgTreeRow { id: 5, label: "110 омбр".into(), full_name: None, parent_id: Some(4) },
        ];
        // org_editor з 17 АК (id=2): видимі — 2 і 3 (своя + піддерево), НЕ видно 1/4/5.
        let visible = [2, 3];
        let restricted = restrict_tree(rows, &visible);

        assert_eq!(restricted.len(), 2, "мають лишитись лише 17 АК і 241 обр ТрО");
        assert!(restricted.iter().all(|r| r.id == 2 || r.id == 3), "жодного чужого вузла");

        let seventeenth_ak = restricted.iter().find(|r| r.id == 2).unwrap();
        assert_eq!(
            seventeenth_ak.parent_id, None,
            "своя організація стає коренем для рендеру, бо справжній батько (id=1) невидимий"
        );
        let child = restricted.iter().find(|r| r.id == 3).unwrap();
        assert_eq!(child.parent_id, Some(2), "видимий батько лишається як є");
    }
}
