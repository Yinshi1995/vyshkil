use leptos::prelude::*;

use crate::types::actor::Actor;

/// Генерує D1 (05 §D1) для одного дня й повертає готові байти xlsx напряму — той самий підхід, що
/// решта server fn цього проєкту (клієнт нічого не читає з диска), браузерне скачування —
/// `components::download_bytes` на клієнті. Копія лишається на диску + рядок `generated_document`
/// (05 §вступ) для майбутнього перегляду історії генерацій (Етап 7+, ще не побудовано).
#[server(GenerateD1, "/api")]
pub async fn generate_d1(
    actor: Option<Actor>,
    org_id: i32,
    as_of_date: String,
) -> Result<Vec<u8>, ServerFnError> {
    use crate::backend::{documents, policy, repo};

    let actor = actor.ok_or_else(|| ServerFnError::new("оберіть актора вгорі"))?;
    let db = expect_context::<sea_orm::DatabaseConnection>();

    if !policy::can_view_org(&db, actor, org_id).await.map_err(|e| ServerFnError::new(e.to_string()))? {
        return Err(ServerFnError::new("немає права переглядати дані цієї частини"));
    }

    let date = chrono::NaiveDate::parse_from_str(&as_of_date, "%Y-%m-%d")
        .map_err(|_| ServerFnError::new("«станом на»: неможлива дата"))?;

    let org_label = repo::documents::org_label(&db, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("частину не знайдено"))?;

    let rollup = repo::documents::daily_training_rollup(&db, org_id, &as_of_date)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    documents::d1::build_day_sheet(&mut workbook, &org_label, date, &rollup)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let bytes =
        workbook.save_to_buffer().map_err(|e| ServerFnError::new(e.to_string()))?;

    // Копія на диск (01 §5 "оригінали на диску") -- не блокує повернення байтів клієнту, якщо
    // запис не вдався: сама генерація важливіша за журнал генерацій цього першого зрізу.
    let dir = std::env::var("DOCUMENTS_DIR").unwrap_or_else(|_| "data/generated_documents".into());
    let dir_path = std::path::Path::new(&dir).join("d1");
    if std::fs::create_dir_all(&dir_path).is_ok() {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let file_path = dir_path.join(format!("{org_id}_{as_of_date}_{suffix}.xlsx"));
        if std::fs::write(&file_path, &bytes).is_ok() {
            let _ = repo::documents::insert_generated_document(
                &db,
                "d1",
                org_id,
                &as_of_date,
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
