//! Інтеграційний тест мапінгу облікових записів при вході через ІКС (12-auth.md §2.5) —
//! repo::auth на реальному Postgres з усіма міграціями (070 `oidc_sub` включно) + чисте
//! рішення `oidc::decide_mapping`. Потребує `TEST_DATABASE_URL`; без нього — пропущено
//! (той самий підхід, що `tests/relay.rs`).

use app::backend::repo::auth;
use migration::MigratorTrait;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, FromQueryResult, Statement};
use server::oidc::{decide_mapping, Mapping};
use uuid::Uuid;

/// Дзеркалить `tests/relay.rs::fresh_test_db` (свідоме дублювання, див. коментар там).
async fn fresh_test_db() -> Option<DatabaseConnection> {
    let Ok(test_url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("TEST_DATABASE_URL не задано — інтеграційний тест server::oidc_mapping пропущено");
        return None;
    };
    let slash = test_url.rfind('/').expect("TEST_DATABASE_URL має бути виду postgres://.../ім'я_бази");
    let base_name = &test_url[slash + 1..];
    assert!(
        !base_name.is_empty() && base_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "ім'я тестової бази має містити лише [a-zA-Z0-9_]: {base_name:?}"
    );
    let db_name = format!("{base_name}_{}", Uuid::now_v7().simple());
    let admin_url = format!("{}/postgres", &test_url[..slash]);
    let admin_db = Database::connect(&admin_url).await.expect("maintenance-база 'postgres'");
    admin_db.execute_unprepared(&format!("DROP DATABASE IF EXISTS {db_name}")).await.expect("drop");
    admin_db.execute_unprepared(&format!("CREATE DATABASE {db_name}")).await.expect("create");
    let db = Database::connect(&format!("{}/{db_name}", &test_url[..slash])).await.expect("з'єднання");
    migration::Migrator::up(&db, None).await.expect("міграції на тестовій базі");
    Some(db)
}

#[derive(FromQueryResult, Debug)]
struct Row {
    login: String,
    is_active: bool,
    must_change_password: bool,
    display_name: Option<String>,
    first_name: Option<String>,
    rank: Option<String>,
    phone: Option<String>,
    delta_nick: Option<String>,
    oidc_sub: Option<String>,
}

async fn row(db: &DatabaseConnection, id: i32) -> Row {
    Row::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT login, is_active, must_change_password, display_name, first_name, rank, phone, \
                delta_nick, oidc_sub FROM user_account WHERE id = $1",
        [id.into()],
    ))
    .one(db)
    .await
    .unwrap()
    .unwrap()
}

#[tokio::test]
async fn first_login_creates_inactive_viewer_then_binds_and_syncs() {
    let Some(db) = fresh_test_db().await else { return };

    // Невідомий логін → створюємо.
    assert!(auth::find_user_for_sso(&db, "petrenko").await.unwrap().is_none());
    assert_eq!(decide_mapping(None, None, "sub-1"), Mapping::Create);
    let id = auth::create_sso_user(&db, "petrenko", "sub-1").await.unwrap();

    let r = row(&db, id).await;
    assert!(!r.is_active, "новий акаунт з ІКС чекає адміністратора");
    assert!(!r.must_change_password);
    assert_eq!(r.oidc_sub.as_deref(), Some("sub-1"));

    // Роль viewer — в org dev-сід-адміна (міграція 047).
    let roles = auth::user_roles(&db, id).await.unwrap();
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0].role, "viewer");
    let admin = auth::find_user_for_sso(&db, "admin").await.unwrap().expect("dev-сід admin");
    let admin_roles = auth::user_roles(&db, admin.id).await.unwrap();
    assert!(admin_roles.iter().any(|r| r.role == "admin" && r.org_id == roles[0].org_id));

    // Пароль для такого акаунта неможливий (хеш-заглушка).
    let acc = auth::find_user_by_login(&db, "petrenko").await.unwrap().unwrap();
    assert_eq!(acc.password_hash, "!sso");

    // Пошук без урахування регістру + той самий sub → наявний, без повторної прив'язки.
    let found = auth::find_user_for_sso(&db, "PETRENKO").await.unwrap().expect("case-insensitive");
    assert_eq!(found.id, id);
    let owner = auth::find_user_id_by_oidc_sub(&db, "sub-1").await.unwrap();
    assert_eq!(decide_mapping(Some(&found), owner, "sub-1"), Mapping::Existing { user_id: id, bind: false });
    // Інший профіль ІКС з тим самим логіном → конфлікт.
    assert_eq!(decide_mapping(Some(&found), None, "sub-OTHER"), Mapping::Conflict);

    // Синхронізація профілю: порожні claims очищають поле, display_name без claim не чіпаємо.
    let p = auth::SsoProfile {
        display_name: Some("Іван Петренко".into()),
        first_name: Some("Іван".into()),
        rank: Some("капітан".into()),
        delta_nick: Some("sokil".into()),
        ..Default::default()
    };
    auth::sync_sso_profile(&db, id, &p).await.unwrap();
    let r = row(&db, id).await;
    assert_eq!(r.display_name.as_deref(), Some("Іван Петренко"));
    assert_eq!(r.first_name.as_deref(), Some("Іван"));
    assert_eq!(r.rank.as_deref(), Some("капітан"));
    assert_eq!(r.delta_nick.as_deref(), Some("sokil"));
    auth::sync_sso_profile(&db, id, &auth::SsoProfile { rank: Some("майор".into()), ..Default::default() })
        .await
        .unwrap();
    let r = row(&db, id).await;
    assert_eq!(r.rank.as_deref(), Some("майор"));
    assert_eq!(r.first_name, None);
    assert_eq!(r.phone, None);
    assert_eq!(r.display_name.as_deref(), Some("Іван Петренко"));
    assert_eq!(r.login, "petrenko");
}

#[tokio::test]
async fn existing_password_account_gets_bound_once() {
    let Some(db) = fresh_test_db().await else { return };

    let admin = auth::find_user_for_sso(&db, "Admin").await.unwrap().expect("dev-сід admin");
    assert!(admin.oidc_sub.is_none());
    assert_eq!(decide_mapping(Some(&admin), None, "sub-a"), Mapping::Existing { user_id: admin.id, bind: true });
    auth::bind_oidc_sub(&db, admin.id, "sub-a").await.unwrap();
    // Повторна прив'язка іншим sub не перезаписує першу.
    auth::bind_oidc_sub(&db, admin.id, "sub-b").await.unwrap();
    assert_eq!(row(&db, admin.id).await.oidc_sub.as_deref(), Some("sub-a"));

    // sub прив'язаний до admin, а логін в ІКС уже інший → конфлікт, не дубль.
    let owner = auth::find_user_id_by_oidc_sub(&db, "sub-a").await.unwrap();
    assert_eq!(owner, Some(admin.id));
    assert_eq!(decide_mapping(None, owner, "sub-a"), Mapping::Conflict);
}
