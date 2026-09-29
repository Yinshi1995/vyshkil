//! SQL/SeaORM для словників Етапу 2: `vos`, `equipment`, `equipment_vos`, `position`,
//! `vos_position` — довідникові дані, доступні будь-якому актору (не org-scoped, на відміну
//! від `repo::orgs`, тому тут немає `policy`-фільтрації, як і в `services::orgs::list_orgs`).
//! Виняток — `learned_aliases`/`confirm_learned_alias`/`reject_learned_alias`: підтвердження
//! learned-синонімів — дія адміна (перевіряється в `pages/dictionaries/server.rs`).

use crate::domain::normalize::normalize;
use crate::types::dictionaries::{
    DictionariesOverview, DictionaryEntry, EquipmentVosHint, LearnedAlias,
};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};

/// Підказка "ОВТ/сленг → ВОС" (02 §3): той самий `alias`+`pg_trgm` патерн, що й `repo::orgs::
/// search_orgs`, тільки `target_type = 'equipment'` і приєднання через `equipment_vos` до `vos`.
/// Ранжування — так само: точний збіг синоніма → вага (`equipment_vos.weight`) → схожість.
pub async fn equipment_vos_hint(
    db: &DatabaseConnection,
    query: &str,
) -> Result<Vec<EquipmentVosHint>, DbErr> {
    let norm_query = normalize(query);
    if norm_query.is_empty() {
        return Ok(Vec::new());
    }

    #[derive(FromQueryResult)]
    struct Row {
        vos_code: String,
        vos_title: String,
        equipment_name: String,
        matched_raw: String,
        is_exact: bool,
    }

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"
        WITH matches AS (
            SELECT
                v.code AS vos_code,
                v.title AS vos_title,
                e.name AS equipment_name,
                a.raw AS matched_raw,
                ev.weight,
                (a.norm = $1) AS is_exact,
                similarity(a.norm, $1) AS sim
            FROM alias a
            JOIN equipment e ON e.id = a.target_id AND a.target_type = 'equipment'
            JOIN equipment_vos ev ON ev.equipment_id = e.id
            JOIN vos v ON v.id = ev.vos_id
            WHERE a.norm = $1 OR a.norm % $1
        ),
        ranked AS (
            SELECT
                *,
                ROW_NUMBER() OVER (
                    PARTITION BY vos_code
                    ORDER BY is_exact DESC, weight DESC, sim DESC
                ) AS rn
            FROM matches
        )
        SELECT vos_code, vos_title, equipment_name, matched_raw, is_exact
        FROM ranked
        WHERE rn = 1
        ORDER BY is_exact DESC, weight DESC, sim DESC
        LIMIT 10
        "#,
        [norm_query.into()],
    );

    let rows = Row::find_by_statement(stmt).all(db).await?;

    Ok(rows
        .into_iter()
        .map(|r| EquipmentVosHint {
            vos_code: r.vos_code,
            vos_title: r.vos_title,
            matched_equipment: r.equipment_name,
            matched_raw: r.matched_raw,
            is_exact: r.is_exact,
        })
        .collect())
}

#[derive(FromQueryResult)]
struct SimpleRow {
    id: i32,
    label: String,
    extra: Option<String>,
}

async fn simple_list(db: &DatabaseConnection, sql: &str) -> Result<Vec<DictionaryEntry>, DbErr> {
    let stmt = Statement::from_string(db.get_database_backend(), sql);
    let rows = SimpleRow::find_by_statement(stmt).all(db).await?;
    Ok(rows.into_iter().map(|r| DictionaryEntry { id: r.id, label: r.label, extra: r.extra }).collect())
}

/// Усі "прості" довідники Етапу 2 одним викликом (01 §2) — для сторінки адмінки.
pub async fn dictionaries_overview(db: &DatabaseConnection) -> Result<DictionariesOverview, DbErr> {
    Ok(DictionariesOverview {
        training_kinds: simple_list(
            db,
            "SELECT id, name AS label, code AS extra FROM training_kind \
             WHERE deleted_at IS NULL ORDER BY name",
        )
        .await?,
        training_directions: simple_list(
            db,
            "SELECT id, name AS label, code AS extra FROM training_direction \
             WHERE deleted_at IS NULL ORDER BY name",
        )
        .await?,
        bzvp_programs: simple_list(
            db,
            "SELECT id, name AS label, NULL::text AS extra FROM bzvp_program \
             WHERE deleted_at IS NULL ORDER BY name",
        )
        .await?,
        vos: simple_list(
            db,
            "SELECT id, (code || ' — ' || title) AS label, status AS extra FROM vos \
             WHERE deleted_at IS NULL ORDER BY code",
        )
        .await?,
        positions: simple_list(
            db,
            "SELECT id, name AS label, NULL::text AS extra FROM \"position\" \
             WHERE deleted_at IS NULL ORDER BY name",
        )
        .await?,
        equipment: simple_list(
            db,
            "SELECT id, name AS label, category AS extra FROM equipment \
             WHERE deleted_at IS NULL ORDER BY name",
        )
        .await?,
        courses: simple_list(
            db,
            "SELECT id, name AS label, NULL::text AS extra FROM course \
             WHERE deleted_at IS NULL ORDER BY name",
        )
        .await?,
        attrition_reasons: simple_list(
            db,
            "SELECT id, name AS label, \
                    (CASE WHEN requires_note THEN 'потребує примітки' ELSE NULL END) AS extra \
             FROM attrition_reason WHERE deleted_at IS NULL ORDER BY name",
        )
        .await?,
    })
}

/// Learned-синоніми, що чекають підтвердження адміном (01 §"Навчання") — найстаріші перші.
pub async fn learned_aliases(db: &DatabaseConnection) -> Result<Vec<LearnedAlias>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
        target_type: String,
        raw: String,
        norm: String,
        uses_count: i32,
        created_at: String,
    }

    let stmt = Statement::from_string(
        db.get_database_backend(),
        "SELECT id, target_type, raw, norm, uses_count, \
                to_char(created_at, 'YYYY-MM-DD HH24:MI') AS created_at \
         FROM alias WHERE source = 'learned' ORDER BY created_at",
    );
    let rows = Row::find_by_statement(stmt).all(db).await?;

    Ok(rows
        .into_iter()
        .map(|r| LearnedAlias {
            id: r.id,
            target_type: r.target_type,
            raw: r.raw,
            norm: r.norm,
            uses_count: r.uses_count,
            created_at: r.created_at,
        })
        .collect())
}

/// Підтвердити learned-синонім: залишається в `alias`, стає `source = 'manual'` (адмін підтвердив
/// відповідність — те саме довірче джерело, що й ручне введення). Транзакція з `SET LOCAL
/// app.actor` — щоб audit_log знав, хто підтвердив.
pub async fn confirm_learned_alias(db: &impl ConnectionTrait, alias_id: i32) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE alias SET source = 'manual' WHERE id = $1 AND source = 'learned'",
        [alias_id.into()],
    ))
    .await?;
    Ok(())
}

/// Відхилити learned-синонім: видаляється з `alias` (хибна відповідність — не тримаємо її).
pub async fn reject_learned_alias(db: &impl ConnectionTrait, alias_id: i32) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "DELETE FROM alias WHERE id = $1 AND source = 'learned'",
        [alias_id.into()],
    ))
    .await?;
    Ok(())
}

/// ВОС за кодом точно (Етап 5, 03 §4/§5): джерела зберігають ВОС як код, не вільний текст --
/// нечіткий пошук тут зайвий. `None` — "ВОС не з довідника" (03 §5, помилка).
pub async fn resolve_vos_by_code(
    db: &DatabaseConnection,
    code: &str,
) -> Result<Option<(i32, String)>, DbErr> {
    let code = code.trim();
    if code.is_empty() {
        return Ok(None);
    }
    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
        code: String,
        title: String,
    }
    let row = Row::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, code, title FROM vos WHERE code = $1 AND deleted_at IS NULL",
        [code.into()],
    ))
    .one(db)
    .await?;
    Ok(row.map(|r| (r.id, format!("{} — {}", r.code, r.title))))
}

/// Посада нечітким пошуком по `alias` (Етап 5) — той самий патерн, що й `repo::orgs::search_orgs`,
/// беремо лише найкращий збіг.
pub async fn resolve_position(
    db: &DatabaseConnection,
    raw: &str,
) -> Result<Option<(i32, String)>, DbErr> {
    let norm_query = normalize(raw);
    if norm_query.is_empty() {
        return Ok(None);
    }
    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
        name: String,
    }
    let row = Row::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"
        SELECT p.id, p.name
        FROM alias a
        JOIN "position" p ON p.id = a.target_id AND a.target_type = 'position'
        WHERE a.norm = $1 OR a.norm % $1
        ORDER BY (a.norm = $1) DESC, a.uses_count DESC, similarity(a.norm, $1) DESC
        LIMIT 1
        "#,
        [norm_query.into()],
    ))
    .one(db)
    .await?;
    Ok(row.map(|r| (r.id, r.name)))
}

/// Курс за точною назвою (Етап 5, ІВС) — назви в джерелі (КІБР/КПК/СККВ/…) збігаються з
/// `course.name` буквально (сід із 01 §2), точний регістронезалежний збіг достатній, без
/// `alias`/нечіткого пошуку (на відміну від `resolve_position` — там реальні синоніми в джерелі).
pub async fn resolve_course(db: &DatabaseConnection, raw: &str) -> Result<Option<(i32, String)>, DbErr> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
        name: String,
    }
    let row = Row::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, name FROM course WHERE deleted_at IS NULL AND lower(name) = lower($1)",
        [raw.into()],
    ))
    .one(db)
    .await?;
    Ok(row.map(|r| (r.id, r.name)))
}

/// `training_kind.id` за фіксованим кодом (Етап 5) — файли одного типу завжди одного виду
/// підготовки ("Фах" → `code='special'`), не потребує пошуку.
pub async fn training_kind_id_by_code(db: &DatabaseConnection, code: &str) -> Result<Option<i32>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
    }
    let row = Row::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id FROM training_kind WHERE code = $1 AND deleted_at IS NULL",
        [code.into()],
    ))
    .one(db)
    .await?;
    Ok(row.map(|r| r.id))
}
