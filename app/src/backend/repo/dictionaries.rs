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
use serde::{Deserialize, Serialize};

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
    Ok(rows
        .into_iter()
        .map(|r| DictionaryEntry {
            id: r.id,
            label: r.label,
            extra: r.extra,
            extra_id: None,
        })
        .collect())
}

#[derive(FromQueryResult)]
struct TrainingSiteRow {
    id: i32,
    label: String,
    extra: Option<String>,
    extra_id: i32,
}

async fn training_site_list(db: &DatabaseConnection) -> Result<Vec<DictionaryEntry>, DbErr> {
    let stmt = Statement::from_string(
        db.get_database_backend(),
        "SELECT ts.id, \
                COALESCE(ts.locality, o.short_name) AS label, \
                o.short_name AS extra, \
                ts.org_id AS extra_id \
         FROM training_site ts \
         JOIN org o ON o.id = ts.org_id \
         ORDER BY o.short_name, ts.locality",
    );
    let rows = TrainingSiteRow::find_by_statement(stmt).all(db).await?;
    Ok(rows
        .into_iter()
        .map(|r| DictionaryEntry {
            id: r.id,
            label: r.label,
            extra: r.extra,
            extra_id: Some(r.extra_id),
        })
        .collect())
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
        training_sites: training_site_list(db).await?,
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

// ---------------------------------------------------------------------------
// Admin CRUD: VOS
// ---------------------------------------------------------------------------

pub async fn create_vos(
    db: &impl ConnectionTrait,
    code: &str,
    title: &str,
) -> Result<i32, DbErr> {
    let row = db
        .query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT INTO vos (code, title, status, source) VALUES ($1, $2, 'draft', 'manual') RETURNING id",
            [code.into(), title.into()],
        ))
        .await?
        .ok_or(DbErr::RecordNotFound("vos".into()))?;
    row.try_get::<i32>("", "id")
}

pub async fn update_vos(
    db: &impl ConnectionTrait,
    id: i32,
    code: &str,
    title: &str,
) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE vos SET code = $2, title = $3 WHERE id = $1 AND deleted_at IS NULL",
        [id.into(), code.into(), title.into()],
    ))
    .await?;
    Ok(())
}

pub async fn delete_vos(db: &impl ConnectionTrait, id: i32) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE vos SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
        [id.into()],
    ))
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Admin CRUD: Equipment (ОВТ)
// ---------------------------------------------------------------------------

pub async fn create_equipment(
    db: &impl ConnectionTrait,
    name: &str,
    category: Option<&str>,
) -> Result<i32, DbErr> {
    let row = if let Some(cat) = category {
        db.query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT INTO equipment (name, category) VALUES ($1, $2) RETURNING id",
            [name.into(), cat.into()],
        ))
        .await?
    } else {
        db.query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT INTO equipment (name) VALUES ($1) RETURNING id",
            [name.into()],
        ))
        .await?
    };
    let row = row.ok_or(DbErr::RecordNotFound("equipment".into()))?;
    row.try_get::<i32>("", "id")
}

pub async fn update_equipment(
    db: &impl ConnectionTrait,
    id: i32,
    name: &str,
    category: Option<&str>,
) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE equipment SET name = $2, category = $3 WHERE id = $1 AND deleted_at IS NULL",
        [id.into(), name.into(), category.unwrap_or("").into()],
    ))
    .await?;
    Ok(())
}

pub async fn delete_equipment(db: &impl ConnectionTrait, id: i32) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE equipment SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
        [id.into()],
    ))
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Generic CRUD for simple dictionaries (code+name / name-only / name+bool)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictKind {
    TrainingKind,
    TrainingDirection,
    BzvpProgram,
    Position,
    Course,
    AttritionReason,
}

impl DictKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "training_kind" | "training-kind" => Some(Self::TrainingKind),
            "training_direction" | "training-direction" => Some(Self::TrainingDirection),
            "bzvp_program" | "bzvp-program" => Some(Self::BzvpProgram),
            "position" => Some(Self::Position),
            "course" => Some(Self::Course),
            "attrition_reason" | "attrition-reason" => Some(Self::AttritionReason),
            _ => None,
        }
    }

    fn table(self) -> &'static str {
        match self {
            Self::TrainingKind => "training_kind",
            Self::TrainingDirection => "training_direction",
            Self::BzvpProgram => "bzvp_program",
            Self::Position => "\"position\"",
            Self::Course => "course",
            Self::AttritionReason => "attrition_reason",
        }
    }

    pub fn has_code(self) -> bool {
        matches!(self, Self::TrainingKind | Self::TrainingDirection)
    }

    pub fn has_requires_note(self) -> bool {
        matches!(self, Self::AttritionReason)
    }
}

pub async fn create_dict(
    db: &impl ConnectionTrait,
    kind: DictKind,
    name: &str,
    code: Option<&str>,
    requires_note: Option<bool>,
) -> Result<i32, DbErr> {
    let table = kind.table();
    let sql = if kind.has_code() {
        format!("INSERT INTO {table} (name, code) VALUES ($1, $2) RETURNING id")
    } else if kind.has_requires_note() {
        format!("INSERT INTO {table} (name, requires_note) VALUES ($1, $2) RETURNING id")
    } else {
        format!("INSERT INTO {table} (name) VALUES ($1) RETURNING id")
    };

    let row = if kind.has_code() {
        db.query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            &sql,
            [name.into(), code.unwrap_or("").into()],
        ))
        .await?
    } else if kind.has_requires_note() {
        db.query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            &sql,
            [name.into(), requires_note.unwrap_or(false).into()],
        ))
        .await?
    } else {
        db.query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            &sql,
            [name.into()],
        ))
        .await?
    };
    let row = row.ok_or(DbErr::RecordNotFound(table.into()))?;
    row.try_get::<i32>("", "id")
}

pub async fn update_dict(
    db: &impl ConnectionTrait,
    kind: DictKind,
    id: i32,
    name: &str,
    code: Option<&str>,
    requires_note: Option<bool>,
) -> Result<(), DbErr> {
    let table = kind.table();
    let sql = if kind.has_code() {
        format!("UPDATE {table} SET name = $2, code = $3 WHERE id = $1 AND deleted_at IS NULL")
    } else if kind.has_requires_note() {
        format!("UPDATE {table} SET name = $2, requires_note = $3 WHERE id = $1 AND deleted_at IS NULL")
    } else {
        format!("UPDATE {table} SET name = $2 WHERE id = $1 AND deleted_at IS NULL")
    };

    if kind.has_code() {
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            &sql,
            [id.into(), name.into(), code.unwrap_or("").into()],
        ))
        .await?;
    } else if kind.has_requires_note() {
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            &sql,
            [id.into(), name.into(), requires_note.unwrap_or(false).into()],
        ))
        .await?;
    } else {
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            &sql,
            [id.into(), name.into()],
        ))
        .await?;
    }
    Ok(())
}

pub async fn delete_dict(
    db: &impl ConnectionTrait,
    kind: DictKind,
    id: i32,
) -> Result<(), DbErr> {
    let table = kind.table();
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        format!("UPDATE {table} SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL"),
        [id.into()],
    ))
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Admin CRUD: Training Sites (місця підготовки)
// ---------------------------------------------------------------------------

pub async fn create_training_site(
    db: &impl ConnectionTrait,
    org_id: i32,
    locality: &str,
) -> Result<i32, DbErr> {
    let row = db
        .query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT INTO training_site (org_id, locality) VALUES ($1, $2) RETURNING id",
            [org_id.into(), locality.into()],
        ))
        .await?
        .ok_or(DbErr::RecordNotFound("training_site".into()))?;
    row.try_get::<i32>("", "id")
}

pub async fn update_training_site(
    db: &impl ConnectionTrait,
    id: i32,
    org_id: i32,
    locality: &str,
) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE training_site SET org_id = $2, locality = $3 WHERE id = $1",
        [id.into(), org_id.into(), locality.into()],
    ))
    .await?;
    Ok(())
}

pub async fn delete_training_site(db: &impl ConnectionTrait, id: i32) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "DELETE FROM training_site WHERE id = $1",
        [id.into()],
    ))
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Export all dictionaries + orgs for import template
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportEnums {
    pub org_codes: Vec<String>,
    pub training_kinds: Vec<String>,
    pub training_directions: Vec<String>,
    pub bzvp_programs: Vec<String>,
    pub courses: Vec<String>,
    pub vos_codes: Vec<String>,
    pub positions: Vec<String>,
    pub equipment: Vec<String>,
    pub attrition_reasons: Vec<String>,
}

pub async fn export_enums_for_template(db: &DatabaseConnection) -> Result<ImportEnums, DbErr> {
    #[derive(FromQueryResult)]
    struct NameRow { name: String }
    #[derive(FromQueryResult)]
    struct CodeNameRow { code: String, name: String }
    #[derive(FromQueryResult)]
    struct VosRow { label: String }
    #[derive(FromQueryResult)]
    struct CodeRow { code: String }

    let org_codes = CodeRow::find_by_statement(Statement::from_string(
        db.get_database_backend(),
        "SELECT (COALESCE(number_kind, '') || number) AS code \
         FROM org WHERE kind = 'military_unit' AND deleted_at IS NULL AND number IS NOT NULL \
         ORDER BY number_kind, number",
    ))
    .all(db)
    .await?
    .into_iter()
    .map(|r| r.code)
    .collect();

    let training_kinds = CodeNameRow::find_by_statement(Statement::from_string(
        db.get_database_backend(),
        "SELECT code, name FROM training_kind WHERE deleted_at IS NULL ORDER BY name",
    ))
    .all(db)
    .await?
    .into_iter()
    .map(|r| format!("{} ({})", r.name, r.code))
    .collect();

    let training_directions = NameRow::find_by_statement(Statement::from_string(
        db.get_database_backend(),
        "SELECT name FROM training_direction WHERE deleted_at IS NULL ORDER BY name",
    ))
    .all(db)
    .await?
    .into_iter()
    .map(|r| r.name)
    .collect();

    let bzvp_programs = NameRow::find_by_statement(Statement::from_string(
        db.get_database_backend(),
        "SELECT name FROM bzvp_program WHERE deleted_at IS NULL ORDER BY name",
    ))
    .all(db)
    .await?
    .into_iter()
    .map(|r| r.name)
    .collect();

    let courses = NameRow::find_by_statement(Statement::from_string(
        db.get_database_backend(),
        "SELECT name FROM course WHERE deleted_at IS NULL ORDER BY name",
    ))
    .all(db)
    .await?
    .into_iter()
    .map(|r| r.name)
    .collect();

    let vos_codes = VosRow::find_by_statement(Statement::from_string(
        db.get_database_backend(),
        "SELECT (code || ' — ' || title) AS label FROM vos WHERE deleted_at IS NULL ORDER BY code",
    ))
    .all(db)
    .await?
    .into_iter()
    .map(|r| r.label)
    .collect();

    let positions = NameRow::find_by_statement(Statement::from_string(
        db.get_database_backend(),
        "SELECT name FROM \"position\" WHERE deleted_at IS NULL ORDER BY name",
    ))
    .all(db)
    .await?
    .into_iter()
    .map(|r| r.name)
    .collect();

    let equipment = NameRow::find_by_statement(Statement::from_string(
        db.get_database_backend(),
        "SELECT name FROM equipment WHERE deleted_at IS NULL ORDER BY name",
    ))
    .all(db)
    .await?
    .into_iter()
    .map(|r| r.name)
    .collect();

    let attrition_reasons = NameRow::find_by_statement(Statement::from_string(
        db.get_database_backend(),
        "SELECT name FROM attrition_reason WHERE deleted_at IS NULL ORDER BY name",
    ))
    .all(db)
    .await?
    .into_iter()
    .map(|r| r.name)
    .collect();

    Ok(ImportEnums {
        org_codes,
        training_kinds,
        training_directions,
        bzvp_programs,
        courses,
        vos_codes,
        positions,
        equipment,
        attrition_reasons,
    })
}
