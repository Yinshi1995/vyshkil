//! SQL-агрегація для документів (Етап 7, 05): "правило групування" (01 §1) — які органи
//! потрапляють у "основну" секцію звіту, які в "поза смугою" — і денний rollup БЗВП/Фахова/
//! Адаптація по кожному з них. Сама арифметика воронки — `domain::counting` (як і `repo::groups`);
//! тут лише SQL-вибірка й розкладання по секціях/видах підготовки.
//!
//! `internship` (стажування, з ІВС) свідомо ВИКЛЮЧЕНО з rollup — D1 (05 §D1, еталон
//! `source_files/Зразок/26.09/`) має рівно три колонки БЗВП/Фахова/Адаптація, стажування там
//! немає окремою колонкою.

use crate::domain::counting::{events_on, in_training, EventType, GroupEventRecord};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KindCounts {
    pub total: i64,
    pub finishing_today: i64,
    pub started_today: i64,
}

#[derive(Debug, Clone)]
pub struct OrgRollupRow {
    pub org_id: i32,
    pub org_label: String,
    pub bzvp: KindCounts,
    pub special: KindCounts,
    pub adaptation: KindCounts,
    /// Примітки дня — наявні `group_event.note` за подіями з `occurred_on = D` цього органу,
    /// зібрані "; "-роздільником (той самий стиль, що в еталоні); текст пишеться людиною при
    /// внесенні події, тут нічого не синтезується.
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DailyRollup {
    pub main: Vec<OrgRollupRow>,
    pub out_of_zone: Vec<OrgRollupRow>,
}

/// Правило групування (01 §1, буквально процитоване в 05 §D1): "основна" секція — пряма
/// оперативна дитина `corps_org_id` на `as_of`, ПЛЮС пряма штатна дитина без оперативного батька
/// на `as_of` і статусом `in_zone`. "Поза смугою" — пряма штатна дитина, чий оперативний батько на
/// `as_of` — хтось інший, АБО статус `out_of_zone`. Відсутність рядка `org_status` на дату =
/// `in_zone` за замовчуванням (org_status веде лише винятки, 01 §"org_status").
async fn grouped_org_ids(
    db: &DatabaseConnection,
    corps_org_id: i32,
    as_of: &str,
) -> Result<(Vec<(i32, String)>, Vec<(i32, String)>), DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
        short_name: String,
        number: Option<String>,
        out_of_zone: bool,
    }

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"
        WITH staff_children AS (
            SELECT sc.descendant_id AS id
            FROM subordination_closure sc
            WHERE sc.ancestor_id = $1 AND sc.axis = 'staff' AND sc.depth = 1
              AND daterange(sc.valid_from, sc.valid_to, '[)') @> $2::date
        ),
        operational_parent AS (
            SELECT sc.descendant_id AS id, sc.ancestor_id AS parent_id
            FROM subordination_closure sc
            WHERE sc.axis = 'operational' AND sc.depth = 1
              AND daterange(sc.valid_from, sc.valid_to, '[)') @> $2::date
        ),
        current_status AS (
            SELECT DISTINCT ON (os.org_id) os.org_id, os.status
            FROM org_status os
            WHERE daterange(os.valid_from, os.valid_to, '[)') @> $2::date
            ORDER BY os.org_id, os.valid_from DESC
        )
        SELECT
            o.id, o.short_name, o.number,
            (
                sf.id IS NOT NULL
                AND (
                    (op.parent_id IS NOT NULL AND op.parent_id != $1)
                    OR COALESCE(cs.status, 'in_zone') = 'out_of_zone'
                )
            ) AS out_of_zone
        FROM org o
        LEFT JOIN staff_children sf ON sf.id = o.id
        LEFT JOIN operational_parent op ON op.id = o.id
        LEFT JOIN current_status cs ON cs.org_id = o.id
        WHERE o.deleted_at IS NULL
          AND (op.parent_id = $1 OR sf.id IS NOT NULL)
        ORDER BY o.short_name
        "#,
        [corps_org_id.into(), as_of.into()],
    );

    let rows = Row::find_by_statement(stmt).all(db).await?;

    let mut main = Vec::new();
    let mut out_of_zone = Vec::new();
    for r in rows {
        let label = match r.number {
            Some(n) => format!("{} ({n})", r.short_name),
            None => r.short_name,
        };
        if r.out_of_zone {
            out_of_zone.push((r.id, label));
        } else {
            main.push((r.id, label));
        }
    }
    Ok((main, out_of_zone))
}

/// Одна `training_group` з тим, що потрібно для rollup: чий орган, який вид підготовки.
struct RollupGroup {
    id: i32,
    sender_org_id: i32,
    training_kind_code: String,
}

async fn rollup_groups(
    db: &DatabaseConnection,
    org_ids: &[i32],
) -> Result<Vec<RollupGroup>, DbErr> {
    if org_ids.is_empty() {
        return Ok(Vec::new());
    }
    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
        sender_org_id: i32,
        code: String,
    }
    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"
        SELECT tg.id, tg.sender_org_id, tk.code
        FROM training_group tg
        JOIN training_kind tk ON tk.id = tg.training_kind_id
        WHERE tg.sender_org_id = ANY($1) AND tk.code IN ('bzvp', 'special', 'adaptation')
        "#,
        [org_ids.to_vec().into()],
    );
    let rows = Row::find_by_statement(stmt).all(db).await?;
    Ok(rows
        .into_iter()
        .map(|r| RollupGroup { id: r.id, sender_org_id: r.sender_org_id, training_kind_code: r.code })
        .collect())
}

async fn group_events_plain(
    db: &impl ConnectionTrait,
    group_id: i32,
) -> Result<Vec<GroupEventRecord>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        event_type: String,
        count: i32,
        occurred_on: String,
        recorded_at: String,
    }
    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"
        SELECT event_type, count, to_char(occurred_on, 'YYYY-MM-DD') AS occurred_on,
               to_char(recorded_at, 'YYYY-MM-DD"T"HH24:MI:SS') AS recorded_at
        FROM group_event WHERE group_id = $1 ORDER BY occurred_on, id
        "#,
        [group_id.into()],
    );
    let rows = Row::find_by_statement(stmt).all(db).await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| {
            EventType::parse(&r.event_type).map(|event_type| GroupEventRecord {
                event_type,
                count: r.count,
                occurred_on: r.occurred_on,
                recorded_at: r.recorded_at,
            })
        })
        .collect())
}

/// Примітки дня (05 §D1) — наявні `group_event.note` за подіями цього дня, по органу.
async fn notes_for_orgs_on_date(
    db: &DatabaseConnection,
    org_ids: &[i32],
    as_of: &str,
) -> Result<std::collections::HashMap<i32, Vec<String>>, DbErr> {
    if org_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    #[derive(FromQueryResult)]
    struct Row {
        sender_org_id: i32,
        note: String,
    }
    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"
        SELECT tg.sender_org_id, ge.note
        FROM group_event ge
        JOIN training_group tg ON tg.id = ge.group_id
        WHERE tg.sender_org_id = ANY($1) AND ge.occurred_on = $2::date AND ge.note IS NOT NULL
        ORDER BY tg.sender_org_id, ge.id
        "#,
        [org_ids.to_vec().into(), as_of.into()],
    );
    let rows = Row::find_by_statement(stmt).all(db).await?;
    let mut map: std::collections::HashMap<i32, Vec<String>> = std::collections::HashMap::new();
    for r in rows {
        map.entry(r.sender_org_id).or_default().push(r.note);
    }
    Ok(map)
}

fn kind_counts(events: &[GroupEventRecord], as_of: &str) -> KindCounts {
    KindCounts {
        total: in_training(events, as_of, None),
        finishing_today: events_on(events, EventType::Completed, as_of, None),
        started_today: events_on(events, EventType::Started, as_of, None),
    }
}

/// Денний rollup БЗВП/Фахова/Адаптація по прямих (за правилом групування) підрозділах
/// `corps_org_id` на дату `as_of` (`YYYY-MM-DD`) — вхід для D1 (`backend::documents::d1`).
pub async fn daily_training_rollup(
    db: &DatabaseConnection,
    corps_org_id: i32,
    as_of: &str,
) -> Result<DailyRollup, DbErr> {
    let (main_orgs, out_of_zone_orgs) = grouped_org_ids(db, corps_org_id, as_of).await?;
    let all_org_ids: Vec<i32> =
        main_orgs.iter().chain(out_of_zone_orgs.iter()).map(|(id, _)| *id).collect();

    let groups = rollup_groups(db, &all_org_ids).await?;
    let notes = notes_for_orgs_on_date(db, &all_org_ids, as_of).await?;

    // events по кожній групі — окремим запитом (report-генерація, не гарячий шлях; той самий
    // підхід, що repo::groups::group_events, лише без Leptos-контексту тут).
    let mut events_by_group: std::collections::HashMap<i32, Vec<GroupEventRecord>> =
        std::collections::HashMap::new();
    for g in &groups {
        events_by_group.insert(g.id, group_events_plain(db, g.id).await?);
    }

    let build_row = |org_id: i32, org_label: String| -> OrgRollupRow {
        let mut row = OrgRollupRow {
            org_id,
            org_label,
            bzvp: KindCounts::default(),
            special: KindCounts::default(),
            adaptation: KindCounts::default(),
            note: notes.get(&org_id).map(|ns| ns.join("; ")),
        };
        for g in groups.iter().filter(|g| g.sender_org_id == org_id) {
            let events = events_by_group.get(&g.id).map(Vec::as_slice).unwrap_or_default();
            let counts = kind_counts(events, as_of);
            match g.training_kind_code.as_str() {
                "bzvp" => merge_counts(&mut row.bzvp, counts),
                "special" => merge_counts(&mut row.special, counts),
                "adaptation" => merge_counts(&mut row.adaptation, counts),
                _ => {}
            }
        }
        row
    };

    Ok(DailyRollup {
        main: main_orgs.into_iter().map(|(id, label)| build_row(id, label)).collect(),
        out_of_zone: out_of_zone_orgs.into_iter().map(|(id, label)| build_row(id, label)).collect(),
    })
}

fn merge_counts(acc: &mut KindCounts, add: KindCounts) {
    acc.total += add.total;
    acc.finishing_today += add.finishing_today;
    acc.started_today += add.started_today;
}

/// "<short_name> (<номер>)" органу, для якого генерується документ (заголовок аркуша) — `None`,
/// якщо `org_id` не існує/видалено (сторінка не мала б дозволити такий виклик, але сервер не
/// вірить клієнту на слово).
pub async fn org_label(db: &DatabaseConnection, org_id: i32) -> Result<Option<String>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        short_name: String,
        number: Option<String>,
    }
    let row = Row::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT short_name, number FROM org WHERE id = $1 AND deleted_at IS NULL",
        [org_id.into()],
    ))
    .one(db)
    .await?;
    Ok(row.map(|r| match r.number {
        Some(n) => format!("{} ({n})", r.short_name),
        None => r.short_name,
    }))
}

/// Записує рядок в `generated_document` (05 §вступ: "кожен згенерований документ зберігається") —
/// `file_path` уже записаний на диск викликачем (`pages/documents/server.rs`) ДО цього виклику.
pub async fn insert_generated_document(
    db: &DatabaseConnection,
    kind: &str,
    org_id: i32,
    as_of_date: &str,
    file_path: &str,
) -> Result<i32, DbErr> {
    #[derive(FromQueryResult)]
    struct NewId {
        id: i32,
    }
    let row = NewId::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO generated_document (kind, org_id, as_of_date, file_path) \
         VALUES ($1, $2, $3::date, $4) RETURNING id",
        [kind.into(), org_id.into(), as_of_date.into(), file_path.into()],
    ))
    .one(db)
    .await?
    .ok_or_else(|| DbErr::Custom("INSERT generated_document не повернув id".into()))?;
    Ok(row.id)
}
