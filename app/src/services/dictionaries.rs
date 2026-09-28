use leptos::prelude::*;

use crate::types::dictionaries::DictionariesOverview;

/// Усі "прості" довідники Етапу 2 (01 §2) — не org-scoped, доступно будь-кому. Потрібен ≥ 2
/// сторінкам (`pages::dictionaries`, `pages::training_form` — заповнити `<select>` виду
/// підготовки/програми БЗВП) — тому тут, а не в `pages/<p>/server.rs` (07 §3.5).
#[server(GetDictionariesOverview, "/api")]
pub async fn get_dictionaries_overview() -> Result<DictionariesOverview, ServerFnError> {
    use crate::backend::repo;

    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::dictionaries::dictionaries_overview(&db)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
