//! Інтеграційний тест модуля `personnel` (`.claude/decisions/personnel-named-persons.md`) — окрема тестова БД
//! (див. `common`). Перевіряє: прив'язку особи до події, дедуп, ліміт осіб ≤ count, зняття прив'язки,
//! журнал доступу й — головне — що ПІБ НЕ потрапляють в `audit_log` та `personnel_access_log`.
#![cfg(feature = "ssr")]

mod common;

use app::backend::repo::personnel::{
    event_group_id, link_person_to_event, list_event_persons, unlink_person, NewPerson,
};
use common::fresh_test_db;
use sea_orm::{ConnectionTrait, FromQueryResult, Statement};

#[derive(FromQueryResult)]
struct Ev {
    id: i32,
    count: i32,
}

#[derive(FromQueryResult)]
struct N {
    n: i64,
}

fn person(last: &str, first: &str) -> NewPerson {
    NewPerson {
        rank: Some("сержант".into()),
        last_name: last.into(),
        first_name: first.into(),
        birth_date: Some("1990-05-17".into()),
        extra: None,
    }
}

async fn count(db: &sea_orm::DatabaseConnection, sql: &str) -> i64 {
    N::find_by_statement(Statement::from_string(db.get_database_backend(), sql.to_string()))
        .one(db)
        .await
        .unwrap()
        .unwrap()
        .n
}

#[tokio::test]
async fn personnel_link_dedupe_limit_and_no_pii_in_logs() {
    let Some(db) = fresh_test_db().await else { return };
    let be = db.get_database_backend();
    validation_rejects_bad_input_before_touching_db(&db).await;

    // подія з к-тю ≥ 3 з dev-сіду
    let ev = Ev::find_by_statement(Statement::from_string(
        be,
        "SELECT id, count FROM group_event WHERE count BETWEEN 3 AND 50 ORDER BY id LIMIT 1",
    ))
    .one(&db)
    .await
    .unwrap()
    .expect("у dev-сіді має бути подія з к-тю ≥ 3");
    let other = Ev::find_by_statement(Statement::from_sql_and_values(
        be,
        "SELECT id, count FROM group_event WHERE id <> $1 AND count >= 1 ORDER BY id LIMIT 1",
        [ev.id.into()],
    ))
    .one(&db)
    .await
    .unwrap()
    .expect("потрібна друга подія");
    assert!(event_group_id(&db, ev.id).await.unwrap().is_some());
    assert!(event_group_id(&db, -1).await.unwrap().is_none());

    // 1. створення + прив'язка
    let (p1, created) = link_person_to_event(&db, ev.id, &person("УнікальнеПрізвищеАльфа", "Іван"), None, "1:admin")
        .await
        .unwrap();
    assert!(created);
    assert_eq!(p1.birth_date.as_deref(), Some("1990-05-17"));

    // 2. та сама особа в ту саму подію — відмова (без значень у повідомленні)
    let dup = link_person_to_event(&db, ev.id, &person("унікальнепрізвищеальфа", "ІВАН"), None, "1:admin").await;
    let msg = format!("{:?}", dup.unwrap_err());
    assert!(msg.contains("вже додана") && !msg.contains("Альфа") && !msg.contains("альфа"), "повідомлення без ПІБ: {msg}");

    // 3. та сама особа в ІНШУ подію — перевикористання запису (дедуп за нормалізованим ключем)
    let (p1b, created_b) = link_person_to_event(&db, other.id, &person("УНІКАЛЬНЕПРІЗВИЩЕАЛЬФА", "іван"), None, "1:admin")
        .await
        .unwrap();
    assert!(!created_b);
    assert_eq!(p1b.id, p1.id);
    assert_eq!(count(&db, "SELECT count(*) AS n FROM person").await, 1);

    // 4. ліміт: осіб не більше, ніж count події
    let mut linked = 1;
    let mut i = 0;
    while linked < ev.count {
        i += 1;
        link_person_to_event(&db, ev.id, &person(&format!("Тест{i}"), "Петро"), None, "1:admin").await.unwrap();
        linked += 1;
    }
    let over = link_person_to_event(&db, ev.id, &person("Зайвий", "Особа"), None, "1:admin").await;
    assert!(over.is_err(), "осіб більше, ніж count — відмова");

    // 5. список + зняття прив'язки (сама особа лишається — вона ще в іншій події)
    let listed = list_event_persons(&db, ev.id, "1:admin").await.unwrap();
    assert_eq!(listed.len() as i32, ev.count);
    assert!(unlink_person(&db, ev.id, p1.id, "1:admin").await.unwrap());
    assert!(!unlink_person(&db, ev.id, p1.id, "1:admin").await.unwrap(), "повторне зняття — false");
    assert_eq!(count(&db, &format!("SELECT count(*) AS n FROM group_event_person WHERE person_id = {}", p1.id)).await, 1);

    // 6. ПІБ не потрапили в audit_log і в журнал доступу
    assert_eq!(
        count(&db, "SELECT count(*) AS n FROM audit_log WHERE coalesce(after::text,'') || coalesce(before::text,'') ILIKE '%Альфа%' OR table_name IN ('person','group_event_person')").await,
        0,
        "audit_log не має містити особових даних"
    );
    assert_eq!(
        count(&db, "SELECT count(*) AS n FROM personnel_access_log WHERE actor !~ '^[0-9]+:[a-z_]+$'").await,
        0,
        "журнал доступу: актор лише org:role"
    );
    // створення, прив'язки, читання й зняття залишили слід лише з ідентифікаторами
    assert!(count(&db, "SELECT count(*) AS n FROM personnel_access_log").await >= 5);
}

/// Частина основного тесту: тести одного файлу ділять одну тестову БД (`fresh_test_db` її перестворює),
/// тому валідація перевіряється тут же, а не окремим `#[tokio::test]`.
async fn validation_rejects_bad_input_before_touching_db(db: &sea_orm::DatabaseConnection) {
    let before = count(db, "SELECT count(*) AS n FROM person").await;
    let ev = Ev::find_by_statement(Statement::from_string(
        db.get_database_backend(),
        "SELECT id, count FROM group_event WHERE count >= 1 ORDER BY id LIMIT 1",
    ))
    .one(db)
    .await
    .unwrap()
    .unwrap();
    let mut bad = person("", "Іван");
    assert!(link_person_to_event(db, ev.id, &bad, None, "1:admin").await.is_err());
    bad = person("Іваненко", "Іван");
    bad.birth_date = Some("1850-01-01".into());
    assert!(link_person_to_event(db, ev.id, &bad, None, "1:admin").await.is_err());
    assert_eq!(count(db, "SELECT count(*) AS n FROM person").await, before, "некоректний ввід не створює запис");
    // неіснуюча подія
    assert!(link_person_to_event(db, -5, &person("Коваль", "Іван"), None, "1:admin").await.is_err());
}
