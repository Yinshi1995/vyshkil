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
