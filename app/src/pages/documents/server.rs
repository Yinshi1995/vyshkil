use leptos::prelude::*;

use crate::types::actor::Actor;

/// Генерує D1 (05 §D1) — повний тижневий файл для органу: 7 денних аркушів + службові
/// "початок"/"кінець" (приховані маркери для Excel 3D-формул) + "тижневий" аркуш
/// `SUM(початок:кінець!C5)`. `any_day_in_week` — будь-який день потрібного тижня (сервер
/// сам рахує пн–нд). Той самий підхід, що `generate_d2` — клієнт не читає диск,
/// `components::download_bytes` ініціює браузерне скачування.
#[server(GenerateD1, "/api")]
pub async fn generate_d1(
    actor: Option<Actor>,
    org_id: i32,
    any_day_in_week: String,
) -> Result<Vec<u8>, ServerFnError> {
    use crate::backend::{documents, policy, repo};
    use chrono::Datelike;

    let actor = actor.ok_or_else(|| ServerFnError::new("оберіть актора вгорі"))?;
    let db = expect_context::<sea_orm::DatabaseConnection>();

    if !policy::can_view_org(&db, actor, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
    {
        return Err(ServerFnError::new("немає права переглядати дані цієї частини"));
    }

    let any_day = chrono::NaiveDate::parse_from_str(&any_day_in_week, "%Y-%m-%d")
        .map_err(|_| ServerFnError::new("оберіть дату"))?;
    let week_start =
        any_day - chrono::Duration::days(any_day.weekday().num_days_from_monday() as i64);

    let org_label = repo::documents::org_label(&db, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("частину не знайдено"))?;

    let mut days = Vec::with_capacity(7);
    for offset in 0..7 {
        let date = week_start + chrono::Duration::days(offset);
        let date_str = date.format("%Y-%m-%d").to_string();
        let rollup = repo::documents::daily_training_rollup(&db, org_id, &date_str)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        days.push((date, rollup));
    }

    let mut workbook = rust_xlsxwriter::Workbook::new();
    documents::d1::build_weekly_file(&mut workbook, &org_label, week_start, &days)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let bytes = workbook.save_to_buffer().map_err(|e| ServerFnError::new(e.to_string()))?;

    let dir = std::env::var("DOCUMENTS_DIR").unwrap_or_else(|_| "data/generated_documents".into());
    let dir_path = std::path::Path::new(&dir).join("d1");
    if std::fs::create_dir_all(&dir_path).is_ok() {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let week_start_str = week_start.format("%Y-%m-%d").to_string();
        let file_path = dir_path.join(format!("{org_id}_{week_start_str}_{suffix}.xlsx"));
        if std::fs::write(&file_path, &bytes).is_ok() {
            let _ = repo::documents::insert_generated_document(
                &db,
                "d1",
                org_id,
                &week_start_str,
                &file_path.to_string_lossy(),
            )
            .await;
        }
    }

    Ok(bytes)
}

/// Генерує D2 ("Контролька", 05 §D2) для тижня, що містить `any_day_in_week` (пн..нд) — та сама
/// схема, що `generate_d1`, лише правило групування об'єднане в один плаский список на корпус
/// (`backend::documents::d2` doc-comment пояснює чому, на реальних даних еталона) і формули
/// трирівневі. `org_id` у `generated_document` — корінь ієрархії (`repo::documents::root_org_id`):
/// D2 не має "власника"-органу, як D1, охоплює все одразу.
#[server(GenerateD2, "/api")]
pub async fn generate_d2(
    actor: Option<Actor>,
    any_day_in_week: String,
) -> Result<Vec<u8>, ServerFnError> {
    use crate::backend::{documents, policy, repo};
    use chrono::Datelike;

    let actor = actor.ok_or_else(|| ServerFnError::new("оберіть актора вгорі"))?;
    let db = expect_context::<sea_orm::DatabaseConnection>();

    let any_day = chrono::NaiveDate::parse_from_str(&any_day_in_week, "%Y-%m-%d")
        .map_err(|_| ServerFnError::new("оберіть дату"))?;
    let week_start = any_day - chrono::Duration::days(any_day.weekday().num_days_from_monday() as i64);

    let corps = repo::documents::top_level_orgs(&db, &any_day_in_week)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    for (corps_id, _) in &corps {
        if !policy::can_view_org(&db, actor, *corps_id)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?
        {
            return Err(ServerFnError::new("немає права переглядати дані одного з корпусів"));
        }
    }

    let mut days = Vec::with_capacity(7);
    for offset in 0..7 {
        let date = week_start + chrono::Duration::days(offset);
        let date_str = date.format("%Y-%m-%d").to_string();
        let mut day_corps = Vec::with_capacity(corps.len());
        for (corps_id, corps_label) in &corps {
            let rollup = repo::documents::daily_training_rollup(&db, *corps_id, &date_str)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            if rollup.main.is_empty() && rollup.out_of_zone.is_empty() {
                continue;
            }
            let mut orgs = rollup.main;
            orgs.extend(rollup.out_of_zone);
            day_corps.push((corps_label.clone(), orgs));
        }
        days.push(documents::d2::DayBlock { date, corps: day_corps });
    }

    let mut workbook = rust_xlsxwriter::Workbook::new();
    documents::d2::build_week_sheet(&mut workbook, week_start, &days)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let bytes = workbook.save_to_buffer().map_err(|e| ServerFnError::new(e.to_string()))?;

    let root_id = repo::documents::root_org_id(&db, &any_day_in_week)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    if let Some(root_id) = root_id {
        let dir = std::env::var("DOCUMENTS_DIR").unwrap_or_else(|_| "data/generated_documents".into());
        let dir_path = std::path::Path::new(&dir).join("d2");
        if std::fs::create_dir_all(&dir_path).is_ok() {
            let suffix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default();
            let week_start_str = week_start.format("%Y-%m-%d").to_string();
            let file_path = dir_path.join(format!("{week_start_str}_{suffix}.xlsx"));
            if std::fs::write(&file_path, &bytes).is_ok() {
                let _ = repo::documents::insert_generated_document(
                    &db,
                    "d2",
                    root_id,
                    &week_start_str,
                    &file_path.to_string_lossy(),
                )
                .await;
            }
        }
    }

    Ok(bytes)
}

/// Генерує D3 "Говорілка" (05 §D3) — docx з текстом доповіді за один день.
#[server(GenerateD3, "/api")]
pub async fn generate_d3(
    actor: Option<Actor>,
    as_of_date: String,
) -> Result<Vec<u8>, ServerFnError> {
    use crate::backend::{documents, policy, repo};

    let actor = actor.ok_or_else(|| ServerFnError::new("оберіть актора вгорі"))?;
    let db = expect_context::<sea_orm::DatabaseConnection>();

    let today = chrono::NaiveDate::parse_from_str(&as_of_date, "%Y-%m-%d")
        .map_err(|_| ServerFnError::new("оберіть дату"))?;
    let yesterday = today - chrono::Duration::days(1);

    let corps = repo::documents::top_level_orgs(&db, &as_of_date)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    for (corps_id, _) in &corps {
        if !policy::can_view_org(&db, actor, *corps_id)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?
        {
            return Err(ServerFnError::new("немає права переглядати дані одного з корпусів"));
        }
    }

    let today_str = today.format("%Y-%m-%d").to_string();
    let yesterday_str = yesterday.format("%Y-%m-%d").to_string();
    let mut corps_days = Vec::with_capacity(corps.len());
    for (corps_id, corps_label) in &corps {
        let rollup_today = repo::documents::daily_training_rollup(&db, *corps_id, &today_str)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        let rollup_yesterday = repo::documents::daily_training_rollup(&db, *corps_id, &yesterday_str)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        corps_days.push(documents::d3::CorpsDay {
            label: corps_label.clone(),
            today: rollup_today,
            yesterday: rollup_yesterday,
        });
    }

    let template_path = std::env::var("DOCUMENTS_D3_TEMPLATE").ok();
    let template_text = template_path
        .as_ref()
        .and_then(|p| std::fs::read_to_string(p).ok());
    let template = template_text.as_deref().unwrap_or(documents::d3::DEFAULT_TEMPLATE);

    let paragraphs = documents::d3::render_paragraphs(template, today, &corps_days);
    let bytes = documents::d3::build_docx(&paragraphs)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let root_id = repo::documents::root_org_id(&db, &as_of_date)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    if let Some(root_id) = root_id {
        let dir = std::env::var("DOCUMENTS_DIR").unwrap_or_else(|_| "data/generated_documents".into());
        let dir_path = std::path::Path::new(&dir).join("d3");
        if std::fs::create_dir_all(&dir_path).is_ok() {
            let suffix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default();
            let file_path = dir_path.join(format!("{as_of_date}_{suffix}.docx"));
            if std::fs::write(&file_path, &bytes).is_ok() {
                let _ = repo::documents::insert_generated_document(
                    &db,
                    "d3",
                    root_id,
                    &as_of_date,
                    &file_path.to_string_lossy(),
                )
                .await;
            }
        }
    }

    Ok(bytes)
}

// ---------------------------------------------------------------------------
// D5 — додатки корпусу (Étap 9, 05 §D5)
// ---------------------------------------------------------------------------

#[server(GenerateD5Fah, "/api")]
pub async fn generate_d5_fah(
    actor: Option<Actor>,
    org_id: i32,
    as_of_date: String,
) -> Result<Vec<u8>, ServerFnError> {
    use crate::backend::{documents, policy, repo};

    let actor = actor.ok_or_else(|| ServerFnError::new("оберіть актора вгорі"))?;
    let db = expect_context::<sea_orm::DatabaseConnection>();

    if !policy::can_view_org(&db, actor, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
    {
        return Err(ServerFnError::new("немає права переглядати дані цієї частини"));
    }

    let org_label = repo::documents::org_label(&db, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("частину не знайдено"))?;

    let groups = repo::documents::group_detail_for_corps(&db, org_id, &as_of_date, "special")
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    documents::d5::build_fah(&mut workbook, &org_label, &as_of_date, &groups)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let bytes = workbook.save_to_buffer().map_err(|e| ServerFnError::new(e.to_string()))?;

    save_generated(&db, "d5_fah", org_id, &as_of_date, &bytes).await;
    Ok(bytes)
}

#[server(GenerateD5Bps, "/api")]
pub async fn generate_d5_bps(
    actor: Option<Actor>,
    org_id: i32,
    as_of_date: String,
) -> Result<Vec<u8>, ServerFnError> {
    use crate::backend::{documents, policy, repo};

    let actor = actor.ok_or_else(|| ServerFnError::new("оберіть актора вгорі"))?;
    let db = expect_context::<sea_orm::DatabaseConnection>();

    if !policy::can_view_org(&db, actor, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
    {
        return Err(ServerFnError::new("немає права переглядати дані цієї частини"));
    }

    let org_label = repo::documents::org_label(&db, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("частину не знайдено"))?;

    let groups = repo::documents::group_detail_bps(&db, org_id, &as_of_date)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    documents::d5::build_bps(&mut workbook, &org_label, &as_of_date, &groups)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let bytes = workbook.save_to_buffer().map_err(|e| ServerFnError::new(e.to_string()))?;

    save_generated(&db, "d5_bps", org_id, &as_of_date, &bytes).await;
    Ok(bytes)
}

#[server(GenerateD5Kvid, "/api")]
pub async fn generate_d5_kvid(
    actor: Option<Actor>,
    org_id: i32,
    as_of_date: String,
) -> Result<Vec<u8>, ServerFnError> {
    use crate::backend::{documents, policy, repo};

    let actor = actor.ok_or_else(|| ServerFnError::new("оберіть актора вгорі"))?;
    let db = expect_context::<sea_orm::DatabaseConnection>();

    if !policy::can_view_org(&db, actor, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
    {
        return Err(ServerFnError::new("немає права переглядати дані цієї частини"));
    }

    let org_label = repo::documents::org_label(&db, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("частину не знайдено"))?;

    let rows = repo::documents::staffing_for_corps(&db, org_id, &as_of_date, "squad_leaders")
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    documents::d5::build_kvid(&mut workbook, &org_label, &as_of_date, &rows)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let bytes = workbook.save_to_buffer().map_err(|e| ServerFnError::new(e.to_string()))?;

    save_generated(&db, "d5_kvid", org_id, &as_of_date, &bytes).await;
    Ok(bytes)
}

#[server(GenerateD5Ivs, "/api")]
pub async fn generate_d5_ivs(
    actor: Option<Actor>,
    org_id: i32,
    as_of_date: String,
) -> Result<Vec<u8>, ServerFnError> {
    use crate::backend::{documents, policy, repo};

    let actor = actor.ok_or_else(|| ServerFnError::new("оберіть актора вгорі"))?;
    let db = expect_context::<sea_orm::DatabaseConnection>();

    if !policy::can_view_org(&db, actor, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
    {
        return Err(ServerFnError::new("немає права переглядати дані цієї частини"));
    }

    let org_label = repo::documents::org_label(&db, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("частину не знайдено"))?;

    let rows = repo::documents::staffing_for_corps(&db, org_id, &as_of_date, "instructors")
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    documents::d5::build_ivs(&mut workbook, &org_label, &as_of_date, &rows)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let bytes = workbook.save_to_buffer().map_err(|e| ServerFnError::new(e.to_string()))?;

    save_generated(&db, "d5_ivs", org_id, &as_of_date, &bytes).await;
    Ok(bytes)
}

#[server(GenerateD5Terminy, "/api")]
pub async fn generate_d5_terminy(
    actor: Option<Actor>,
    org_id: i32,
    as_of_date: String,
) -> Result<Vec<u8>, ServerFnError> {
    use crate::backend::{documents, policy, repo};

    let actor = actor.ok_or_else(|| ServerFnError::new("оберіть актора вгорі"))?;
    let db = expect_context::<sea_orm::DatabaseConnection>();

    if !policy::can_view_org(&db, actor, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
    {
        return Err(ServerFnError::new("немає права переглядати дані цієї частини"));
    }

    let org_label = repo::documents::org_label(&db, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("частину не знайдено"))?;

    let (bzvp, special, adaptation) =
        repo::documents::terminy_detail(&db, org_id, &as_of_date)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    documents::d5::build_terminy(&mut workbook, &org_label, &as_of_date, &bzvp, &special, &adaptation)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let bytes = workbook.save_to_buffer().map_err(|e| ServerFnError::new(e.to_string()))?;

    save_generated(&db, "d5_terminy", org_id, &as_of_date, &bytes).await;
    Ok(bytes)
}

// ---------------------------------------------------------------------------
// D6 — передані частини (Étap 9, 05 §D6)
// ---------------------------------------------------------------------------

#[server(GenerateD6, "/api")]
pub async fn generate_d6(
    actor: Option<Actor>,
    as_of_date: String,
) -> Result<Vec<u8>, ServerFnError> {
    use crate::backend::{documents, policy, repo};

    let actor = actor.ok_or_else(|| ServerFnError::new("оберіть актора вгорі"))?;
    let db = expect_context::<sea_orm::DatabaseConnection>();

    let corps = repo::documents::top_level_orgs(&db, &as_of_date)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    for (corps_id, _) in &corps {
        if !policy::can_view_org(&db, actor, *corps_id)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?
        {
            return Err(ServerFnError::new("немає права переглядати дані одного з корпусів"));
        }
    }

    let rows = repo::documents::transferred_orgs_report(&db, &as_of_date)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    documents::d6::build_transferred_report(&mut workbook, &as_of_date, &rows)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let bytes = workbook.save_to_buffer().map_err(|e| ServerFnError::new(e.to_string()))?;

    let root_id = repo::documents::root_org_id(&db, &as_of_date)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    if let Some(root_id) = root_id {
        save_generated(&db, "d6", root_id, &as_of_date, &bytes).await;
    }

    Ok(bytes)
}

/// Зберігає згенерований документ на диск і в `generated_document`.
#[cfg(feature = "ssr")]
async fn save_generated(
    db: &sea_orm::DatabaseConnection,
    kind: &str,
    org_id: i32,
    as_of: &str,
    bytes: &[u8],
) {
    use crate::backend::repo;

    let dir = std::env::var("DOCUMENTS_DIR").unwrap_or_else(|_| "data/generated_documents".into());
    let dir_path = std::path::Path::new(&dir).join(kind);
    if std::fs::create_dir_all(&dir_path).is_ok() {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let ext = "xlsx";
        let file_path = dir_path.join(format!("{org_id}_{as_of}_{suffix}.{ext}"));
        if std::fs::write(&file_path, bytes).is_ok() {
            let _ = repo::documents::insert_generated_document(
                db,
                kind,
                org_id,
                as_of,
                &file_path.to_string_lossy(),
            )
            .await;
        }
    }
}

/// Генерує D4 "Підготовка" (05 §D4) — pptx презентація.
#[server(GenerateD4, "/api")]
pub async fn generate_d4(
    actor: Option<Actor>,
    as_of_date: String,
) -> Result<Vec<u8>, ServerFnError> {
    use crate::backend::{documents, policy, repo};

    let actor = actor.ok_or_else(|| ServerFnError::new("оберіть актора вгорі"))?;
    let db = expect_context::<sea_orm::DatabaseConnection>();

    let today = chrono::NaiveDate::parse_from_str(&as_of_date, "%Y-%m-%d")
        .map_err(|_| ServerFnError::new("оберіть дату"))?;
    let yesterday = today - chrono::Duration::days(1);

    let corps = repo::documents::top_level_orgs(&db, &as_of_date)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    for (corps_id, _) in &corps {
        if !policy::can_view_org(&db, actor, *corps_id)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?
        {
            return Err(ServerFnError::new("немає права переглядати дані одного з корпусів"));
        }
    }

    let today_str = today.format("%Y-%m-%d").to_string();
    let yesterday_str = yesterday.format("%Y-%m-%d").to_string();
    let mut corps_slides = Vec::with_capacity(corps.len());
    for (corps_id, corps_label) in &corps {
        let rollup_today = repo::documents::daily_training_rollup(&db, *corps_id, &today_str)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        let rollup_yesterday = repo::documents::daily_training_rollup(&db, *corps_id, &yesterday_str)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        corps_slides.push(documents::d4::CorpsSlide {
            label: corps_label.clone(),
            today: rollup_today,
            yesterday: rollup_yesterday,
        });
    }

    let bytes = documents::d4::build_pptx(today, &corps_slides)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let root_id = repo::documents::root_org_id(&db, &as_of_date)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    if let Some(root_id) = root_id {
        let dir = std::env::var("DOCUMENTS_DIR").unwrap_or_else(|_| "data/generated_documents".into());
        let dir_path = std::path::Path::new(&dir).join("d4");
        if std::fs::create_dir_all(&dir_path).is_ok() {
            let suffix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default();
            let file_path = dir_path.join(format!("{as_of_date}_{suffix}.pptx"));
            if std::fs::write(&file_path, &bytes).is_ok() {
                let _ = repo::documents::insert_generated_document(
                    &db,
                    "d4",
                    root_id,
                    &as_of_date,
                    &file_path.to_string_lossy(),
                )
                .await;
            }
        }
    }

    Ok(bytes)
}
