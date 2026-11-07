//! Модуль `personnel` (рішення `.claude/decisions/personnel-named-persons.md`): особи поіменно до подій групи.
//!
//! Вимкнений за замовчуванням (`PERSONNEL_ENABLED`), перевірку прапорця й права роблять виклики (`server::personnel_api`).
//! ПРАВИЛА цього модуля:
//! - імена не потрапляють у `audit_log`/логи: журнал доступу (`personnel_access_log`) пише ЛИШЕ ідентифікатори;
//! - повідомлення про помилки не містять значень полів;
//! - `count` події лишається джерелом істини — осіб не може бути більше, ніж `count`.

use sea_orm::{ConnectionTrait, DbErr, FromQueryResult, Statement};
use serde::{Deserialize, Serialize};

use crate::domain::normalize::normalize;

/// Межі року народження (без годинника в `app` — перевірка груба, лише від очевидних помилок вводу).
pub const MIN_BIRTH_YEAR: i32 = 1920;
pub const MAX_BIRTH_YEAR: i32 = 2012;
const MAX_NAME_LEN: usize = 100;
const MAX_RANK_LEN: usize = 60;
const MAX_EXTRA_BYTES: usize = 4096;

#[derive(Debug, Clone, Deserialize)]
pub struct NewPerson {
    pub rank: Option<String>,
    pub last_name: String,
    pub first_name: String,
    /// `YYYY-MM-DD`
    pub birth_date: Option<String>,
    /// Майбутні поля (по батькові, жетон…); має бути JSON-об'єктом.
    #[serde(default)]
    pub extra: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PersonRow {
    pub id: i32,
    pub rank: Option<String>,
    pub last_name: String,
    pub first_name: String,
    pub birth_date: Option<String>,
    pub extra: serde_json::Value,
}

/// Ключ для пошуку дублів: нормалізоване "прізвище|ім'я|дата".
pub fn norm_key(last_name: &str, first_name: &str, birth_date: Option<&str>) -> String {
    format!("{}|{}|{}", normalize(last_name), normalize(first_name), birth_date.unwrap_or(""))
}

/// Перевірка вводу. Повідомлення БЕЗ значень полів (щоб не потрапили в логи).
pub fn validate(p: &NewPerson) -> Result<(), String> {
    let last = p.last_name.trim();
    let first = p.first_name.trim();
    if last.is_empty() || first.is_empty() {
        return Err("Прізвище та ім'я обов'язкові".into());
    }
    if last.chars().count() > MAX_NAME_LEN || first.chars().count() > MAX_NAME_LEN {
        return Err("Прізвище/ім'я задовгі".into());
    }
    if p.rank.as_deref().map(|r| r.trim().chars().count() > MAX_RANK_LEN).unwrap_or(false) {
        return Err("Звання задовге".into());
    }
    if let Some(b) = p.birth_date.as_deref().filter(|b| !b.trim().is_empty()) {
        let d = chrono::NaiveDate::parse_from_str(b.trim(), "%Y-%m-%d")
            .map_err(|_| "Дата народження: формат РРРР-ММ-ДД".to_string())?;
        use chrono::Datelike;
        if d.year() < MIN_BIRTH_YEAR || d.year() > MAX_BIRTH_YEAR {
            return Err("Дата народження поза допустимим діапазоном".into());
        }
    }
    if let Some(e) = &p.extra {
        if !e.is_object() {
            return Err("Додаткові дані мають бути об'єктом".into());
        }
        if e.to_string().len() > MAX_EXTRA_BYTES {
            return Err("Додаткові дані завеликі".into());
        }
    }
    Ok(())
}

#[derive(FromQueryResult)]
struct Row {
    id: i32,
    rank: Option<String>,
    last_name: String,
    first_name: String,
    birth_date: Option<String>,
    extra: String,
}

impl From<Row> for PersonRow {
    fn from(r: Row) -> Self {
        PersonRow {
            id: r.id,
            rank: r.rank,
            last_name: r.last_name,
            first_name: r.first_name,
            birth_date: r.birth_date,
            extra: serde_json::from_str(&r.extra).unwrap_or_else(|_| serde_json::json!({})),
        }
    }
}

const SELECT_PERSON: &str = "SELECT p.id, p.rank, p.last_name, p.first_name, \
     to_char(p.birth_date, 'YYYY-MM-DD') AS birth_date, p.extra::text AS extra FROM person p";

async fn access_log(
    db: &impl ConnectionTrait,
    action: &str,
    person_id: Option<i32>,
    event_id: Option<i32>,
    actor: &str,
) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO personnel_access_log (action, person_id, event_id, actor) VALUES ($1, $2, $3, $4)",
        [action.into(), person_id.into(), event_id.into(), actor.into()],
    ))
    .await?;
    Ok(())
}

/// `group_id` події (для перевірки прав виклику) або `None`, якщо події нема.
pub async fn event_group_id(db: &impl ConnectionTrait, event_id: i32) -> Result<Option<i32>, DbErr> {
    #[derive(FromQueryResult)]
    struct G {
        group_id: i32,
    }
    Ok(G::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT group_id FROM group_event WHERE id = $1",
        [event_id.into()],
    ))
    .one(db)
    .await?
    .map(|g| g.group_id))
}

/// Створює (або знаходить за `norm_key`) особу й прив'язує до події. `Ok((особа, створено_нову))`.
/// Помилки вводу/обмежень — `DbErr::Custom` без значень полів.
pub async fn link_person_to_event(
    db: &impl ConnectionTrait,
    event_id: i32,
    p: &NewPerson,
    created_by: Option<i32>,
    actor: &str,
) -> Result<(PersonRow, bool), DbErr> {
    validate(p).map_err(DbErr::Custom)?;

    #[derive(FromQueryResult)]
    struct Ev {
        count: i32,
    }
    let ev = Ev::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT count FROM group_event WHERE id = $1",
        [event_id.into()],
    ))
    .one(db)
    .await?
    .ok_or_else(|| DbErr::Custom("Події не існує".into()))?;

    #[derive(FromQueryResult)]
    struct Cnt {
        n: i64,
    }
    let linked = Cnt::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT count(*) AS n FROM group_event_person WHERE event_id = $1",
        [event_id.into()],
    ))
    .one(db)
    .await?
    .map(|c| c.n)
    .unwrap_or(0);
    if ev.count > 0 && linked >= ev.count as i64 {
        return Err(DbErr::Custom("Осіб не може бути більше, ніж к-ть у події".into()));
    }

    let last = p.last_name.trim();
    let first = p.first_name.trim();
    let birth = p.birth_date.as_deref().map(str::trim).filter(|b| !b.is_empty());
    let key = norm_key(last, first, birth);

    let existing = Row::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        format!("{SELECT_PERSON} WHERE p.norm_key = $1 AND p.deleted_at IS NULL ORDER BY p.id LIMIT 1"),
        [key.clone().into()],
    ))
    .one(db)
    .await?;

    let (person, created) = match existing {
        Some(r) => (PersonRow::from(r), false),
        None => {
            let extra = p.extra.clone().unwrap_or_else(|| serde_json::json!({})).to_string();
            let rank = p.rank.as_deref().map(str::trim).filter(|r| !r.is_empty());
            let r = Row::find_by_statement(Statement::from_sql_and_values(
                db.get_database_backend(),
                "WITH ins AS (
                    INSERT INTO person (rank, last_name, first_name, birth_date, extra, norm_key, created_by)
                    VALUES ($1, $2, $3, $4::date, $5::jsonb, $6, $7)
                    RETURNING id, rank, last_name, first_name, birth_date, extra
                 )
                 SELECT id, rank, last_name, first_name, to_char(birth_date, 'YYYY-MM-DD') AS birth_date,
                        extra::text AS extra FROM ins",
                [rank.into(), last.into(), first.into(), birth.into(), extra.into(), key.into(), created_by.into()],
            ))
            .one(db)
            .await?
            .ok_or_else(|| DbErr::Custom("Не вдалося створити особу".into()))?;
            (PersonRow::from(r), true)
        }
    };

    let inserted = db
        .execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT INTO group_event_person (event_id, person_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            [event_id.into(), person.id.into()],
        ))
        .await?
        .rows_affected();
    if inserted == 0 {
        return Err(DbErr::Custom("Особа вже додана до цієї події".into()));
    }

    if created {
        access_log(db, "create", Some(person.id), Some(event_id), actor).await?;
    }
    access_log(db, "link", Some(person.id), Some(event_id), actor).await?;
    Ok((person, created))
}

pub async fn list_event_persons(db: &impl ConnectionTrait, event_id: i32, actor: &str) -> Result<Vec<PersonRow>, DbErr> {
    let rows = Row::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        format!(
            "{SELECT_PERSON} JOIN group_event_person gep ON gep.person_id = p.id \
             WHERE gep.event_id = $1 AND p.deleted_at IS NULL ORDER BY p.last_name, p.first_name"
        ),
        [event_id.into()],
    ))
    .all(db)
    .await?;
    access_log(db, "read", None, Some(event_id), actor).await?;
    Ok(rows.into_iter().map(PersonRow::from).collect())
}

/// Знімає прив'язку особи до події (сама особа лишається — може бути в інших подіях). `true`, якщо була.
pub async fn unlink_person(db: &impl ConnectionTrait, event_id: i32, person_id: i32, actor: &str) -> Result<bool, DbErr> {
    let n = db
        .execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "DELETE FROM group_event_person WHERE event_id = $1 AND person_id = $2",
            [event_id.into(), person_id.into()],
        ))
        .await?
        .rows_affected();
    if n > 0 {
        access_log(db, "unlink", Some(person_id), Some(event_id), actor).await?;
    }
    Ok(n > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(last: &str, first: &str, birth: Option<&str>) -> NewPerson {
        NewPerson {
            rank: Some("сержант".into()),
            last_name: last.into(),
            first_name: first.into(),
            birth_date: birth.map(String::from),
            extra: None,
        }
    }

    #[test]
    fn validate_requires_names() {
        assert!(validate(&p("", "Іван", None)).is_err());
        assert!(validate(&p("Шевченко", " ", None)).is_err());
        assert!(validate(&p("Шевченко", "Іван", None)).is_ok());
    }

    #[test]
    fn validate_birth_date() {
        assert!(validate(&p("Т", "І", Some("1990-02-30"))).is_err(), "неіснуюча дата");
        assert!(validate(&p("Т", "І", Some("90-01-01"))).is_err(), "формат");
        assert!(validate(&p("Т", "І", Some("1850-01-01"))).is_err(), "надто давно");
        assert!(validate(&p("Т", "І", Some("2020-01-01"))).is_err(), "надто пізно");
        assert!(validate(&p("Т", "І", Some("1990-05-17"))).is_ok());
        assert!(validate(&p("Т", "І", Some(""))).is_ok(), "порожня дата = не задана");
    }

    #[test]
    fn validate_extra_must_be_object() {
        let mut x = p("Т", "І", None);
        x.extra = Some(serde_json::json!(["a"]));
        assert!(validate(&x).is_err());
        x.extra = Some(serde_json::json!({ "patronymic": "Петрович" }));
        assert!(validate(&x).is_ok());
    }

    #[test]
    fn validate_errors_do_not_leak_values() {
        let e = validate(&p("СекретнеПрізвище", "Ім'я", Some("1850-01-01"))).unwrap_err();
        assert!(!e.contains("Секретне") && !e.contains("1850"), "повідомлення без значень полів: {e}");
    }

    #[test]
    fn norm_key_ignores_case() {
        assert_eq!(norm_key("Петренко", "Олег", Some("1990-01-01")), norm_key("ПЕТРЕНКО", "олег", Some("1990-01-01")));
        assert_ne!(norm_key("Петренко", "Олег", Some("1990-01-01")), norm_key("Петренко", "Олег", Some("1991-01-01")));
    }
}
