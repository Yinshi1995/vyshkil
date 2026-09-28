//! Спільна інфраструктура для інтеграційних тестів (`app/tests/*.rs`) — окрема тестова база в
//! тому ж контейнері Postgres, міграції (включно з dev-сідом) з нуля на кожен прогін
//! ("Як вести розробку далі" §3, docs/spec/00-agent-brief.md).

use migration::MigratorTrait;
use sea_orm::{ConnectionTrait, DatabaseConnection};

/// Потребує `TEST_DATABASE_URL` (напр. `postgres://taktoblik:taktoblik@localhost:5432/taktoblik_test`).
/// Якщо не задано — виклик повертає `None`, і тест-функція має одразу вийти (той самий підхід,
/// що й для `source_files/`-тестів: "тести з ними — skip за відсутності", 00-agent-brief.md).
pub async fn fresh_test_db() -> Option<DatabaseConnection> {
    let Ok(test_url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!(
            "TEST_DATABASE_URL не задано — інтеграційний тест пропущено \
             (див. .claude/memory/MEMORY.md)"
        );
        return None;
    };

    let slash =
        test_url.rfind('/').expect("TEST_DATABASE_URL має бути виду postgres://.../ім'я_бази");
    let db_name = &test_url[slash + 1..];
    assert!(
        !db_name.is_empty() && db_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "ім'я тестової бази має містити лише [a-zA-Z0-9_]: {db_name:?}"
    );
    let admin_url = format!("{}/postgres", &test_url[..slash]);

    let admin_db = sea_orm::Database::connect(&admin_url)
        .await
        .expect("не вдалось з'єднатись з maintenance-базою 'postgres' для перестворення тестової БД");
    admin_db
        .execute_unprepared(&format!(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity \
             WHERE datname = '{db_name}' AND pid <> pg_backend_pid()"
        ))
        .await
        .expect("не вдалось розірвати старі з'єднання з тестовою базою");
    admin_db
        .execute_unprepared(&format!("DROP DATABASE IF EXISTS {db_name}"))
        .await
        .expect("не вдалось видалити стару тестову базу");
    admin_db
        .execute_unprepared(&format!("CREATE DATABASE {db_name}"))
        .await
        .expect("не вдалось створити тестову базу");

    let db = sea_orm::Database::connect(&test_url)
        .await
        .expect("не вдалось з'єднатись зі свіжою тестовою базою");
    migration::Migrator::up(&db, None).await.expect("не вдалось прогнати міграції на тестовій базі");

    Some(db)
}
