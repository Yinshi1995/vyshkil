//! SQL/SeaORM для словників Етапу 2: `vos`, `equipment`, `equipment_vos`, `position`,
//! `vos_position` — довідникові дані, доступні будь-якому актору (не org-scoped, на відміну
//! від `repo::orgs`, тому тут немає `policy`-фільтрації, як і в `services::orgs::list_orgs`).

use crate::domain::normalize::normalize;
use crate::types::dictionaries::EquipmentVosHint;
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
