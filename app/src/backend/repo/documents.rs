//! SQL-агрегація для документів (Етап 7, 05): "правило групування" (01 §1) — які органи
//! потрапляють у "основну" секцію звіту, які в "поза смугою" — і денний rollup БЗВП/Фахова/
//! Адаптація по кожному з них. Сама арифметика воронки — `domain::counting` (як і `repo::groups`);
//! тут лише SQL-вибірка й розкладання по секціях/видах підготовки.
//!
//! `internship` (стажування, з ІВС) свідомо ВИКЛЮЧЕНО з rollup — D1/D2 (05 §D1/§D2, еталони
//! `source_files/Зразок/26.09/`) мають колонки лише БЗВП/Фахова/Адаптація, стажування там
//! немає окремою колонкою.

use crate::domain::counting::{events_on, in_training, EventType, GroupEventRecord};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KindCounts {
    pub total: i64,
    pub finishing_today: i64,
    pub started_today: i64,
    /// "Вибули з різних причин" (05 §D2) — `EventType::Attrition` на цю дату. D1 це поле не читає.
    pub left_today: i64,
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
            o.id, o.short_name,
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
        if r.out_of_zone {
            out_of_zone.push((r.id, r.short_name));
        } else {
            main.push((r.id, r.short_name));
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
        left_today: events_on(events, EventType::Attrition, as_of, None),
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
    acc.left_today += add.left_today;
}

/// Корінь ієрархії на `as_of` — орган без штатного батька, у якого Є штатні діти (відсікає
/// самотні org без підпорядкування, напр. `foreign_state`) — без хардкоду назви "УВ(с) 'Південь'".
pub async fn root_org_id(db: &DatabaseConnection, as_of: &str) -> Result<Option<i32>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
    }
    let row = Row::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"
        SELECT o.id
        FROM org o
        WHERE o.deleted_at IS NULL
          AND NOT EXISTS (
              SELECT 1 FROM subordination_closure sc
              WHERE sc.descendant_id = o.id AND sc.axis = 'staff' AND sc.depth = 1
                AND daterange(sc.valid_from, sc.valid_to, '[)') @> $1::date
          )
          AND EXISTS (
              SELECT 1 FROM subordination_closure sc2
              WHERE sc2.ancestor_id = o.id AND sc2.depth = 1
          )
        LIMIT 1
        "#,
        [as_of.into()],
    ))
    .one(db)
    .await?;
    Ok(row.map(|r| r.id))
}

/// Прямі штатні діти кореня ієрархії — ті самі 6 корпусів/угруповань (17 АК/20 АК/30 КМП/7 КШР/
/// ОТУ "Одеса"/ЧБП), але БЕЗ хардкоду назв (Етап 7, 05 §D2: "Контролька" охоплює всіх одразу,
/// на відміну від D1, де корпус обирає користувач).
pub async fn top_level_orgs(db: &DatabaseConnection, as_of: &str) -> Result<Vec<(i32, String)>, DbErr> {
    let Some(root_id) = root_org_id(db, as_of).await? else { return Ok(Vec::new()) };

    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
        short_name: String,
    }
    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"
        SELECT o.id, o.short_name
        FROM org o
        JOIN subordination_closure sc ON sc.descendant_id = o.id
            AND sc.axis = 'staff' AND sc.depth = 1
            AND daterange(sc.valid_from, sc.valid_to, '[)') @> $2::date
        WHERE sc.ancestor_id = $1 AND o.deleted_at IS NULL
        ORDER BY o.short_name
        "#,
        [root_id.into(), as_of.into()],
    );
    let rows = Row::find_by_statement(stmt).all(db).await?;
    Ok(rows
        .into_iter()
        .map(|r| (r.id, r.short_name))
        .collect())
}

/// Назва органу для заголовків документів — лише short_name (без номеру, ДСК).
pub async fn org_label(db: &DatabaseConnection, org_id: i32) -> Result<Option<String>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        short_name: String,
    }
    let row = Row::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT short_name FROM org WHERE id = $1 AND deleted_at IS NULL",
        [org_id.into()],
    ))
    .one(db)
    .await?;
    Ok(row.map(|r| r.short_name))
}

// ---------------------------------------------------------------------------
// D5 — detail-level queries for corps attachments (Étap 9, 05 §D5)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct GroupDetailRow {
    pub org_label: String,
    pub site_label: String,
    pub vos_code: Option<String>,
    pub position_label: Option<String>,
    pub course_label: Option<String>,
    pub equipment_text: Option<String>,
    pub planned_start: String,
    pub planned_end: String,
    pub in_training: i64,
    pub completed: i64,
    pub planned_count: i64,
}

pub async fn group_detail_for_corps(
    db: &DatabaseConnection,
    corps_org_id: i32,
    as_of: &str,
    training_kind_code: &str,
) -> Result<Vec<GroupDetailRow>, DbErr> {
    let (main_orgs, out_of_zone_orgs) = grouped_org_ids(db, corps_org_id, as_of).await?;
    let all_org_ids: Vec<i32> =
        main_orgs.iter().chain(out_of_zone_orgs.iter()).map(|(id, _)| *id).collect();
    if all_org_ids.is_empty() {
        return Ok(Vec::new());
    }

    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
        org_label: String,
        site_label: String,
        vos_code: Option<String>,
        position_label: Option<String>,
        course_label: Option<String>,
        equipment_text: Option<String>,
        planned_start: String,
        planned_end: String,
    }

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"
        SELECT
            tg.id,
            COALESCE(o.short_name, '') AS org_label,
            COALESCE(ts.name, '') AS site_label,
            v.code AS vos_code,
            p.name AS position_label,
            c.name AS course_label,
            tg.equipment_text,
            to_char(tg.planned_start, 'DD.MM.YYYY') AS planned_start,
            to_char(tg.planned_end, 'DD.MM.YYYY') AS planned_end
        FROM training_group tg
        JOIN training_kind tk ON tk.id = tg.training_kind_id
        JOIN org o ON o.id = tg.sender_org_id
        JOIN training_site ts ON ts.id = tg.site_id
        LEFT JOIN vos v ON v.id = tg.vos_id
        LEFT JOIN "position" p ON p.id = tg.position_id
        LEFT JOIN course c ON c.id = tg.course_id
        WHERE tg.sender_org_id = ANY($1) AND tk.code = $2
        ORDER BY o.short_name, tg.planned_start
        "#,
        [all_org_ids.into(), training_kind_code.into()],
    );

    let rows = Row::find_by_statement(stmt).all(db).await?;

    let mut result = Vec::with_capacity(rows.len());
    for r in rows {
        let events = group_events_plain(db, r.id).await?;
        let in_tr = in_training(&events, as_of, None);
        let compl = events_on(&events, EventType::Completed, as_of, None);
        result.push(GroupDetailRow {
            org_label: r.org_label,
            site_label: r.site_label,
            vos_code: r.vos_code,
            position_label: r.position_label,
            course_label: r.course_label,
            equipment_text: r.equipment_text,
            planned_start: r.planned_start,
            planned_end: r.planned_end,
            in_training: in_tr,
            completed: compl,
            planned_count: in_tr + compl,
        });
    }
    Ok(result)
}

pub async fn bps_vos_ids(db: &DatabaseConnection) -> Result<Vec<i32>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        vos_id: i32,
    }
    let rows = Row::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT DISTINCT ev.vos_id FROM equipment_vos ev",
        [],
    ))
    .all(db)
    .await?;
    Ok(rows.into_iter().map(|r| r.vos_id).collect())
}

pub async fn group_detail_bps(
    db: &DatabaseConnection,
    corps_org_id: i32,
    as_of: &str,
) -> Result<Vec<GroupDetailRow>, DbErr> {
    let all = group_detail_for_corps(db, corps_org_id, as_of, "special").await?;
    let bps_vos = bps_vos_ids(db).await?;
    if bps_vos.is_empty() {
        return Ok(all);
    }

    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
        vos_id: Option<i32>,
    }
    let (main_orgs, out_of_zone_orgs) = grouped_org_ids(db, corps_org_id, as_of).await?;
    let all_org_ids: Vec<i32> =
        main_orgs.iter().chain(out_of_zone_orgs.iter()).map(|(id, _)| *id).collect();
    if all_org_ids.is_empty() {
        return Ok(Vec::new());
    }

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"
        SELECT tg.id, tg.vos_id
        FROM training_group tg
        JOIN training_kind tk ON tk.id = tg.training_kind_id
        WHERE tg.sender_org_id = ANY($1) AND tk.code = 'special'
        "#,
        [all_org_ids.into()],
    );
    let id_rows = Row::find_by_statement(stmt).all(db).await?;
    let bps_group_ids: std::collections::HashSet<i32> = id_rows
        .into_iter()
        .filter(|r| r.vos_id.is_some_and(|v| bps_vos.contains(&v)))
        .map(|r| r.id)
        .collect();

    Ok(all
        .into_iter()
        .enumerate()
        .filter(|(i, _)| bps_group_ids.contains(&(*i as i32)))
        .map(|(_, r)| r)
        .collect())
}

pub async fn terminy_detail(
    db: &DatabaseConnection,
    corps_org_id: i32,
    as_of: &str,
) -> Result<(Vec<GroupDetailRow>, Vec<GroupDetailRow>, Vec<GroupDetailRow>), DbErr> {
    let bzvp = group_detail_for_corps(db, corps_org_id, as_of, "bzvp").await?;
    let special = group_detail_for_corps(db, corps_org_id, as_of, "special").await?;
    let adaptation = group_detail_for_corps(db, corps_org_id, as_of, "adaptation").await?;
    Ok((bzvp, special, adaptation))
}

#[derive(Debug, Clone)]
pub struct StaffingDetail {
    pub org_label: String,
    pub by_tos: i64,
    pub by_list: i64,
    pub present: i64,
    pub trained: i64,
    pub in_training: i64,
    pub planned: i64,
    pub need: i64,
}

pub async fn staffing_for_corps(
    db: &DatabaseConnection,
    corps_org_id: i32,
    as_of: &str,
    category: &str,
) -> Result<Vec<StaffingDetail>, DbErr> {
    let (main_orgs, out_of_zone_orgs) = grouped_org_ids(db, corps_org_id, as_of).await?;
    let all: Vec<(i32, String)> = main_orgs.into_iter().chain(out_of_zone_orgs).collect();
    if all.is_empty() {
        return Ok(Vec::new());
    }
    let org_ids: Vec<i32> = all.iter().map(|(id, _)| *id).collect();

    #[derive(FromQueryResult)]
    struct Row {
        org_id: i32,
        metric: String,
        value: i64,
    }

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"
        SELECT sm.metric, sm.value::bigint, ss.org_id
        FROM staffing_metric sm
        JOIN staffing_snapshot ss ON ss.id = sm.snapshot_id
        WHERE ss.org_id = ANY($1) AND ss.category = $2
          AND ss.as_of = (
              SELECT MAX(s2.as_of) FROM staffing_snapshot s2
              WHERE s2.org_id = ss.org_id AND s2.category = $2 AND s2.as_of <= $3::date
          )
        "#,
        [org_ids.into(), category.into(), as_of.into()],
    );
    let rows = Row::find_by_statement(stmt).all(db).await?;

    let mut map: std::collections::HashMap<i32, StaffingDetail> = std::collections::HashMap::new();
    for (org_id, org_label) in &all {
        map.insert(
            *org_id,
            StaffingDetail {
                org_label: org_label.clone(),
                by_tos: 0,
                by_list: 0,
                present: 0,
                trained: 0,
                in_training: 0,
                planned: 0,
                need: 0,
            },
        );
    }
    for r in rows {
        if let Some(detail) = map.get_mut(&r.org_id) {
            match r.metric.as_str() {
                "by_tos" => detail.by_tos = r.value,
                "by_list" => detail.by_list = r.value,
                "present" => detail.present = r.value,
                "trained_sergeant" => detail.trained = r.value,
                "in_training" => detail.in_training = r.value,
                "planned_next_month" => detail.planned = r.value,
                "need_training" => detail.need = r.value,
                "trained_kibr" => detail.need = r.value, // reuse for IVS
                _ => {}
            }
        }
    }

    Ok(all.iter().filter_map(|(id, _)| map.remove(id)).collect())
}

// ---------------------------------------------------------------------------
// D6 — transferred orgs (Étap 9, 05 §D6)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TransferredOrgRow {
    pub org_label: String,
    pub counterpart_label: Option<String>,
    pub bzvp_count: i64,
    pub special_count: i64,
    pub adaptation_count: i64,
}

pub async fn transferred_orgs_report(
    db: &DatabaseConnection,
    as_of: &str,
) -> Result<Vec<TransferredOrgRow>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        org_id: i32,
        org_label: String,
        counterpart_label: Option<String>,
    }

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"
        SELECT
            o.id AS org_id,
            COALESCE(o.short_name, '') AS org_label,
            co.short_name AS counterpart_label
        FROM org_status os
        JOIN org o ON o.id = os.org_id
        LEFT JOIN org co ON co.id = os.counterpart_org_id
        WHERE os.status = 'transferred'
          AND daterange(os.valid_from, os.valid_to, '[)') @> $1::date
          AND o.deleted_at IS NULL
        ORDER BY o.short_name
        "#,
        [as_of.into()],
    );

    let rows = Row::find_by_statement(stmt).all(db).await?;
    let mut result = Vec::with_capacity(rows.len());

    for r in rows {
        let groups = rollup_groups(db, &[r.org_id]).await?;
        let mut bzvp: i64 = 0;
        let mut special: i64 = 0;
        let mut adaptation: i64 = 0;
        for g in &groups {
            let events = group_events_plain(db, g.id).await?;
            let count = in_training(&events, as_of, None);
            match g.training_kind_code.as_str() {
                "bzvp" => bzvp += count,
                "special" => special += count,
                "adaptation" => adaptation += count,
                _ => {}
            }
        }
        result.push(TransferredOrgRow {
            org_label: r.org_label,
            counterpart_label: r.counterpart_label,
            bzvp_count: bzvp,
            special_count: special,
            adaptation_count: adaptation,
        });
    }
    Ok(result)
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
