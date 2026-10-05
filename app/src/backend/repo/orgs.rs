//! SQL/SeaORM для агрегату "організація": `org`, `alias`, `subordination_closure`,
//! `org_name_history`, `org_status`. Без Leptos-контексту (бере `&DatabaseConnection` явним
//! аргументом), щоб бути тестованим напряму інтеграційними тестами (`app/tests/orgs.rs`).

use crate::domain::normalize::normalize;
use crate::types::org::{
    OrgDetail, OrgHierarchyNode, OrgNumber, OrgSearchResult, OrgTreeRow, SubordinationLink,
};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};

/// Список організацій для перемикача актора й лічильника на головній: (id, назва).
/// Прямий SQL, без entity — Stage 1 ще не заводить повноцінні sea-orm entity для org.
pub async fn list_orgs(db: &DatabaseConnection) -> Result<Vec<(i32, String)>, DbErr> {
    #[derive(FromQueryResult)]
    struct OrgRow {
        id: i32,
        short_name: String,
    }

    let stmt = Statement::from_string(
        db.get_database_backend(),
        "SELECT id, short_name FROM org WHERE deleted_at IS NULL ORDER BY short_name",
    );
    let rows = OrgRow::find_by_statement(stmt).all(db).await?;

    Ok(rows
        .into_iter()
        .map(|r| (r.id, r.short_name))
        .collect())
}

/// Нечіткий пошук організацій по `alias.norm` (02 §3, критерій готовності Етапу 1:
/// "152НЦ"/"а4896"/"польша" знаходять канонічні організації).
/// Запит нормалізується тією ж функцією, що й alias.norm при сіді/введенні (01 §"alias") —
/// інакше "152НЦ" (з великими літерами) не збігся б з засіяним норм-рядком "152нц".
/// Ранжування — за 02 §3: точний збіг синоніма → частота використання цією організацією →
/// схожість (pg_trgm).
pub async fn search_orgs(
    db: &DatabaseConnection,
    query: &str,
) -> Result<Vec<OrgSearchResult>, DbErr> {
    let norm_query = normalize(query);
    if norm_query.is_empty() {
        return default_org_listing(db).await;
    }

    #[derive(FromQueryResult)]
    struct Row {
        org_id: i32,
        short_name: String,
        masked_label: String,
        matched_raw: String,
        is_exact: bool,
    }

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"
        WITH matches AS (
            SELECT
                o.id AS org_id,
                o.short_name,
                COALESCE(o.masked_label, o.short_name) AS masked_label,
                a.raw AS matched_raw,
                a.uses_count,
                (a.norm = $1) AS is_exact,
                similarity(a.norm, $1) AS sim
            FROM alias a
            JOIN org o ON o.id = a.target_id AND a.target_type = 'org'
            WHERE o.deleted_at IS NULL
              AND (a.norm = $1 OR a.norm % $1)
        ),
        ranked AS (
            SELECT
                *,
                ROW_NUMBER() OVER (
                    PARTITION BY org_id
                    ORDER BY is_exact DESC, uses_count DESC, sim DESC
                ) AS rn
            FROM matches
        )
        SELECT org_id, short_name, masked_label, matched_raw, is_exact
        FROM ranked
        WHERE rn = 1
        ORDER BY is_exact DESC, uses_count DESC, sim DESC
        LIMIT 10
        "#,
        [norm_query.into()],
    );

    let rows = Row::find_by_statement(stmt).all(db).await?;

    Ok(rows
        .into_iter()
        .map(|r| OrgSearchResult {
            org_id: r.org_id,
            label: r.short_name,
            masked_label: r.masked_label,
            matched_raw: r.matched_raw,
            is_exact: r.is_exact,
        })
        .collect())
}

/// "Весь довідник" для порожнього запиту (grid-interaction.md §2: "фокус... одразу відкритий
/// список: недавні... далі весь довідник") — викликач (`autocomplete.rs`) додає недавні клієнтом
/// ПЕРЕД цим списком; тут лише прості, бюджетні "перші N за назвою" (не намагаємось відтворити
/// глобальну "найчастіші" статистику через `alias.uses_count` — свідоме спрощення, недавні per-
/// актор уже покривають найчастіший практичний випадок).
async fn default_org_listing(db: &DatabaseConnection) -> Result<Vec<OrgSearchResult>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        org_id: i32,
        short_name: String,
        masked_label: String,
    }
    let stmt = Statement::from_string(
        db.get_database_backend(),
        "SELECT id AS org_id, short_name, COALESCE(masked_label, short_name) AS masked_label FROM org \
         WHERE deleted_at IS NULL ORDER BY short_name LIMIT 30",
    );
    let rows = Row::find_by_statement(stmt).all(db).await?;
    Ok(rows
        .into_iter()
        .map(|r| OrgSearchResult { org_id: r.org_id, label: r.short_name, masked_label: r.masked_label, matched_raw: String::new(), is_exact: false })
        .collect())
}

/// Ведучий числовий префікс нормалізованого рядка ("128 овмбр" → "128"), якщо він є. `None`, якщо
/// рядок не починається з цифр (напр. "чбп", "республіка польща").
fn leading_number(s: &str) -> Option<&str> {
    let len = s.chars().take_while(|c| c.is_ascii_digit()).count();
    (len > 0).then(|| &s[..len])
}

/// Найкращий кандидат для сирого тексту імпорту (Етап 5-6). **Суворіше за `search_orgs`**
/// (яка живить інтерактивний пошук, де людина сама бачить і відкидає слабкі варіанти): коли
/// запит має ведучий номер частини, кандидат приймається лише якщо його номер СПІВПАДАЄ (або
/// точний збіг alias) — інакше `None`. Причина (грабля, спіймана на реальному файлі Етапу 6):
/// pg_trgm-схожість НЕ розрізняє "17 овмбр" від "128 овмбр" (спільний суфікс "овмбр" домінує
/// в короткому рядку, sim 0.583) — і ця схожість ВИЩА за деякі легітимні alias-варіанти
/// ("423 обБпС"/"423 опБпС", sim 0.538), тому проста межа схожості не рятує: потрібен номер.
/// Без ведучого номера в запиті (ЧБП, Республіка Польща) — той самий шлях, що й раніше (перший
/// кандидат `search_orgs`, без додаткової перевірки — там ризик коротко-суфіксної колізії різний).
/// `None` — "частина не розпізнана" (03 §5, помилка блокує фіксацію рядка).
pub async fn resolve_org(db: &DatabaseConnection, raw: &str) -> Result<Option<OrgSearchResult>, DbErr> {
    let norm_query = normalize(raw);
    let Some(query_number) = leading_number(&norm_query) else {
        return Ok(search_orgs(db, raw).await?.into_iter().next());
    };

    #[derive(FromQueryResult)]
    struct Row {
        org_id: i32,
        short_name: String,
        masked_label: String,
        matched_raw: String,
        matched_norm: String,
        is_exact: bool,
    }

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"
        WITH matches AS (
            SELECT
                o.id AS org_id,
                o.short_name,
                COALESCE(o.masked_label, o.short_name) AS masked_label,
                a.raw AS matched_raw,
                a.norm AS matched_norm,
                a.uses_count,
                (a.norm = $1) AS is_exact,
                similarity(a.norm, $1) AS sim
            FROM alias a
            JOIN org o ON o.id = a.target_id AND a.target_type = 'org'
            WHERE o.deleted_at IS NULL
              AND (a.norm = $1 OR a.norm % $1)
        ),
        ranked AS (
            SELECT
                *,
                ROW_NUMBER() OVER (
                    PARTITION BY org_id
                    ORDER BY is_exact DESC, uses_count DESC, sim DESC
                ) AS rn
            FROM matches
        )
        SELECT org_id, short_name, masked_label, matched_raw, matched_norm, is_exact
        FROM ranked
        WHERE rn = 1
        ORDER BY is_exact DESC, uses_count DESC, sim DESC
        LIMIT 10
        "#,
        [norm_query.clone().into()],
    );

    let rows = Row::find_by_statement(stmt).all(db).await?;

    Ok(rows
        .into_iter()
        .find(|r| r.is_exact || leading_number(&r.matched_norm) == Some(query_number))
        .map(|r| OrgSearchResult {
            org_id: r.org_id,
            label: r.short_name,
            masked_label: r.masked_label,
            matched_raw: r.matched_raw,
            is_exact: r.is_exact,
        }))
}

/// Дерево підпорядкування на дату (06-roadmap.md, Етап 1): перемикач осі штатне/оперативне.
/// Гарячий запит — через `subordination_closure` (`depth = 1`, матеріалізоване замикання),
/// **не** рекурсивний CTE (server/CLAUDE.md).
pub async fn subordination_tree(
    db: &DatabaseConnection,
    as_of: &str,
    axis: &str,
) -> Result<Vec<OrgTreeRow>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
        short_name: String,
        full_name: Option<String>,
        parent_id: Option<i32>,
    }

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"
        SELECT o.id, o.short_name, o.full_name, sc.ancestor_id AS parent_id
        FROM org o
        LEFT JOIN subordination_closure sc
            ON sc.descendant_id = o.id
           AND sc.axis = $1
           AND sc.depth = 1
           AND daterange(sc.valid_from, sc.valid_to, '[)') @> $2::date
        WHERE o.deleted_at IS NULL
        ORDER BY o.short_name
        "#,
        [axis.into(), as_of.into()],
    );

    let rows = Row::find_by_statement(stmt).all(db).await?;

    Ok(rows
        .into_iter()
        .map(|r| OrgTreeRow {
            id: r.id,
            label: r.short_name,
            full_name: r.full_name,
            parent_id: r.parent_id,
        })
        .collect())
}

/// Org id + усі його нащадки (subordination_closure, обидві осі, на сьогодні).
pub async fn subtree_org_ids(db: &DatabaseConnection, org_id: i32) -> Result<Vec<i32>, DbErr> {
    #[derive(FromQueryResult)]
    struct IdRow { id: i32 }

    let rows = IdRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT $1::int AS id \
         UNION \
         SELECT descendant_id AS id FROM subordination_closure \
         WHERE ancestor_id = $1 AND daterange(valid_from, valid_to, '[)') @> CURRENT_DATE",
        [org_id.into()],
    ))
    .all(db)
    .await?;

    Ok(rows.into_iter().map(|r| r.id).collect())
}

/// Прямі нащадки (depth=1) організації на сьогодні (обидві осі).
pub async fn direct_children(db: &DatabaseConnection, org_id: i32) -> Result<Vec<(i32, String)>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row { id: i32, label: String }

    let rows = Row::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT o.id, o.short_name AS label \
         FROM subordination_closure sc \
         JOIN org o ON o.id = sc.descendant_id \
         WHERE sc.ancestor_id = $1 AND sc.depth = 1 \
           AND daterange(sc.valid_from, sc.valid_to, '[)') @> CURRENT_DATE \
           AND o.deleted_at IS NULL \
         ORDER BY o.short_name",
        [org_id.into()],
    ))
    .all(db)
    .await?;

    Ok(rows.into_iter().map(|r| (r.id, r.label)).collect())
}

/// Картка частини: поточні дані + історія назв/статусів. `None` — org_id не знайдено
/// (видалено або не існує), щоб виклик міг сам вирішити, як це подати (404 тощо).
pub async fn org_detail(db: &DatabaseConnection, org_id: i32) -> Result<Option<OrgDetail>, DbErr> {
    #[derive(FromQueryResult)]
    struct OrgRow {
        id: i32,
        short_name: String,
        full_name: Option<String>,
        kind: String,
        is_active: bool,
    }
    #[derive(FromQueryResult)]
    struct NameHistoryRow {
        short_name: String,
        valid_from: String,
        valid_to: Option<String>,
    }
    #[derive(FromQueryResult)]
    struct StatusHistoryRow {
        status: String,
        valid_from: String,
        valid_to: Option<String>,
        note: Option<String>,
    }

    let backend = db.get_database_backend();

    let org = OrgRow::find_by_statement(Statement::from_sql_and_values(
        backend,
        "SELECT id, short_name, full_name, kind, is_active \
         FROM org WHERE id = $1 AND deleted_at IS NULL",
        [org_id.into()],
    ))
    .one(db)
    .await?;

    let Some(org) = org else {
        return Ok(None);
    };

    let name_history = NameHistoryRow::find_by_statement(Statement::from_sql_and_values(
        backend,
        "SELECT short_name, to_char(valid_from, 'YYYY-MM-DD') AS valid_from, \
                to_char(valid_to, 'YYYY-MM-DD') AS valid_to \
         FROM org_name_history WHERE org_id = $1 ORDER BY valid_from",
        [org_id.into()],
    ))
    .all(db)
    .await?;

    let status_history = StatusHistoryRow::find_by_statement(Statement::from_sql_and_values(
        backend,
        "SELECT status, to_char(valid_from, 'YYYY-MM-DD') AS valid_from, \
                to_char(valid_to, 'YYYY-MM-DD') AS valid_to, note \
         FROM org_status WHERE org_id = $1 ORDER BY valid_from",
        [org_id.into()],
    ))
    .all(db)
    .await?;

    Ok(Some(OrgDetail {
        id: org.id,
        short_name: org.short_name,
        full_name: org.full_name,
        kind: org.kind,
        is_active: org.is_active,
        name_history: name_history
            .into_iter()
            .map(|r| (r.short_name, r.valid_from, r.valid_to))
            .collect(),
        status_history: status_history
            .into_iter()
            .map(|r| (r.status, r.valid_from, r.valid_to, r.note))
            .collect(),
    }))
}

// ---------------------------------------------------------------------------
// Admin: Hierarchy tree (flat list with parent_id for client-side tree building)
// ---------------------------------------------------------------------------

pub async fn admin_hierarchy(
    db: &DatabaseConnection,
) -> Result<Vec<OrgHierarchyNode>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
        short_name: String,
        kind: String,
        echelon: Option<String>,
        is_active: bool,
        parent_id: Option<i32>,
    }

    let stmt = Statement::from_string(
        db.get_database_backend(),
        "SELECT o.id, o.short_name, o.kind, o.echelon, o.is_active, \
                s.parent_org_id AS parent_id \
         FROM org o \
         LEFT JOIN subordination s ON s.child_org_id = o.id \
             AND s.axis = 'staff' \
             AND (s.valid_to IS NULL OR s.valid_to > NOW()) \
         WHERE o.deleted_at IS NULL \
         ORDER BY o.short_name",
    );
    let rows = Row::find_by_statement(stmt).all(db).await?;
    Ok(rows
        .into_iter()
        .map(|r| OrgHierarchyNode {
            id: r.id,
            short_name: r.short_name,
            kind: r.kind,
            echelon: r.echelon,
            is_active: r.is_active,
            parent_id: r.parent_id,
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Admin: Org CRUD
// ---------------------------------------------------------------------------

pub async fn create_org(
    db: &impl ConnectionTrait,
    short_name: &str,
    kind: &str,
    echelon: Option<&str>,
) -> Result<i32, DbErr> {
    let row = if let Some(ech) = echelon {
        db.query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT INTO org (short_name, kind, echelon) VALUES ($1, $2, $3) RETURNING id",
            [short_name.into(), kind.into(), ech.into()],
        ))
        .await?
    } else {
        db.query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT INTO org (short_name, kind) VALUES ($1, $2) RETURNING id",
            [short_name.into(), kind.into()],
        ))
        .await?
    };
    let row = row.ok_or(DbErr::RecordNotFound("org".into()))?;
    let id: i32 = row.try_get("", "id")?;

    let norm = normalize(short_name);
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO alias (target_type, target_id, raw, norm, source) VALUES ('org', $1, $2, $3, 'manual')",
        [id.into(), short_name.into(), norm.into()],
    ))
    .await?;

    Ok(id)
}

pub async fn update_org(
    db: &impl ConnectionTrait,
    id: i32,
    short_name: &str,
    kind: &str,
    echelon: Option<&str>,
) -> Result<(), DbErr> {
    let ech_val: sea_orm::Value = match echelon {
        Some(e) if !e.is_empty() => e.into(),
        _ => sea_orm::Value::String(None),
    };
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE org SET short_name = $2, kind = $3, echelon = $4 WHERE id = $1 AND deleted_at IS NULL",
        [id.into(), short_name.into(), kind.into(), ech_val],
    ))
    .await?;
    Ok(())
}

pub async fn delete_org(db: &impl ConnectionTrait, id: i32) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE org SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
        [id.into()],
    ))
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Admin: Org Number (separate from name — ДСК)
// ---------------------------------------------------------------------------

pub async fn get_org_number(
    db: &impl ConnectionTrait,
    id: i32,
) -> Result<OrgNumber, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        number_kind: Option<String>,
        number: Option<String>,
    }
    let row = Row::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT number_kind, number FROM org WHERE id = $1 AND deleted_at IS NULL",
        [id.into()],
    ))
    .one(db)
    .await?
    .ok_or(DbErr::RecordNotFound("org".into()))?;

    Ok(OrgNumber {
        number_kind: row.number_kind,
        number: row.number,
    })
}

pub async fn update_org_number(
    db: &impl ConnectionTrait,
    id: i32,
    number_kind: Option<&str>,
    number: Option<&str>,
) -> Result<(), DbErr> {
    let nk_val: sea_orm::Value = match number_kind {
        Some(nk) if !nk.is_empty() => nk.into(),
        _ => sea_orm::Value::String(None),
    };
    let n_val: sea_orm::Value = match number {
        Some(n) if !n.is_empty() => n.into(),
        _ => sea_orm::Value::String(None),
    };
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE org SET number_kind = $2, number = $3 WHERE id = $1 AND deleted_at IS NULL",
        [id.into(), nk_val, n_val],
    ))
    .await?;

    if let Some(num) = number {
        if !num.is_empty() {
            let norm = normalize(num);
            db.execute(Statement::from_sql_and_values(
                db.get_database_backend(),
                "INSERT INTO alias (target_type, target_id, raw, norm, source) \
                 VALUES ('org', $1, $2, $3, 'manual') \
                 ON CONFLICT DO NOTHING",
                [id.into(), num.into(), norm.into()],
            ))
            .await?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Admin: Subordination CRUD
// ---------------------------------------------------------------------------

pub async fn org_subordination_links(
    db: &impl ConnectionTrait,
    org_id: i32,
) -> Result<(Vec<SubordinationLink>, Vec<SubordinationLink>), DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
        other_org_id: i32,
        other_org_name: String,
        axis: String,
        valid_from: String,
        valid_to: Option<String>,
    }

    let backend = db.get_database_backend();

    let parents = Row::find_by_statement(Statement::from_sql_and_values(
        backend,
        "SELECT s.id, s.parent_org_id AS other_org_id, p.short_name AS other_org_name, \
                s.axis, to_char(s.valid_from, 'YYYY-MM-DD') AS valid_from, \
                to_char(s.valid_to, 'YYYY-MM-DD') AS valid_to \
         FROM subordination s \
         JOIN org p ON p.id = s.parent_org_id \
         WHERE s.child_org_id = $1 \
         ORDER BY s.valid_from DESC",
        [org_id.into()],
    ))
    .all(db)
    .await?;

    let children = Row::find_by_statement(Statement::from_sql_and_values(
        backend,
        "SELECT s.id, s.child_org_id AS other_org_id, c.short_name AS other_org_name, \
                s.axis, to_char(s.valid_from, 'YYYY-MM-DD') AS valid_from, \
                to_char(s.valid_to, 'YYYY-MM-DD') AS valid_to \
         FROM subordination s \
         JOIN org c ON c.id = s.child_org_id \
         WHERE s.parent_org_id = $1 AND (s.valid_to IS NULL OR s.valid_to > NOW()) \
         ORDER BY c.short_name",
        [org_id.into()],
    ))
    .all(db)
    .await?;

    let map = |rows: Vec<Row>| -> Vec<SubordinationLink> {
        rows.into_iter()
            .map(|r| SubordinationLink {
                id: r.id,
                other_org_id: r.other_org_id,
                other_org_name: r.other_org_name,
                axis: r.axis,
                valid_from: r.valid_from,
                valid_to: r.valid_to,
            })
            .collect()
    };

    Ok((map(parents), map(children)))
}

pub async fn create_subordination(
    db: &impl ConnectionTrait,
    child_org_id: i32,
    parent_org_id: i32,
    axis: &str,
    valid_from: &str,
) -> Result<i32, DbErr> {
    let row = db
        .query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT INTO subordination (child_org_id, parent_org_id, axis, valid_from) \
             VALUES ($1, $2, $3, $4::date) RETURNING id",
            [
                child_org_id.into(),
                parent_org_id.into(),
                axis.into(),
                valid_from.into(),
            ],
        ))
        .await?
        .ok_or(DbErr::RecordNotFound("subordination".into()))?;
    row.try_get::<i32>("", "id")
}

pub async fn close_subordination(
    db: &impl ConnectionTrait,
    sub_id: i32,
    valid_to: &str,
) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE subordination SET valid_to = $2::date WHERE id = $1",
        [sub_id.into(), valid_to.into()],
    ))
    .await?;
    Ok(())
}

pub async fn delete_subordination(db: &impl ConnectionTrait, sub_id: i32) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "DELETE FROM subordination WHERE id = $1",
        [sub_id.into()],
    ))
    .await?;
    Ok(())
}
