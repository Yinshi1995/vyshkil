//! Регресія безпеки: `update_session_actor` дозволяє перемикати актора лише на роль, яка є в
//! користувача (`user_role`). Раніше будь-який залогінений міг зробити себе адміном через
//! `POST /api/auth/switch-actor` {org_id:1, role:"admin"}. Окрема тестова БД, лише під `ssr`.
#![cfg(feature = "ssr")]

mod common;

use app::backend::repo::auth::{create_session, update_session_actor};
use common::fresh_test_db;
use sea_orm::{ConnectionTrait, FromQueryResult, Statement};

#[derive(FromQueryResult)]
struct IdRow {
    id: i32,
}

#[tokio::test]
async fn session_actor_cannot_be_switched_to_foreign_role() {
    let Some(db) = fresh_test_db().await else { return };
    let be = db.get_database_backend();

    // дві різні org із сіду; користувач — viewer лише в першій
    let orgs: Vec<IdRow> = IdRow::find_by_statement(Statement::from_string(be, "SELECT id FROM org ORDER BY id LIMIT 2"))
        .all(&db)
        .await
        .unwrap();
    assert!(orgs.len() == 2, "у сіді має бути ≥ 2 org");
    let (mine, other) = (orgs[0].id, orgs[1].id);

    let uid = IdRow::find_by_statement(Statement::from_string(
        be,
        "INSERT INTO user_account (login, password_hash, display_name) VALUES ('qa_viewer', 'x', 'QA') RETURNING id",
    ))
    .one(&db)
    .await
    .unwrap()
    .unwrap()
    .id;
    db.execute(Statement::from_sql_and_values(
        be,
        "INSERT INTO user_role (user_id, org_id, role) VALUES ($1, $2, 'viewer')",
        [uid.into(), mine.into()],
    ))
    .await
    .unwrap();

    let sid = create_session(&db, uid, Some(mine), Some("viewer")).await.unwrap();

    // чужа org і підвищена роль — відмова, сесія не змінена
    assert!(!update_session_actor(&db, &sid, other, "viewer").await.unwrap());
    assert!(!update_session_actor(&db, &sid, mine, "admin").await.unwrap());
    assert!(!update_session_actor(&db, &sid, mine, "org_editor").await.unwrap());

    #[derive(FromQueryResult)]
    struct Act {
        active_org_id: Option<i32>,
        active_role: Option<String>,
    }
    let a = Act::find_by_statement(Statement::from_sql_and_values(
        be,
        "SELECT active_org_id, active_role FROM user_session WHERE id = $1::uuid",
        [sid.clone().into()],
    ))
    .one(&db)
    .await
    .unwrap()
    .unwrap();
    assert_eq!((a.active_org_id, a.active_role.as_deref()), (Some(mine), Some("viewer")));

    // власна роль — перемикання дозволено
    assert!(update_session_actor(&db, &sid, mine, "viewer").await.unwrap());
}
