use leptos::prelude::*;

use crate::types::submission::{TrainingSiteOption, VosPositionCourseHint};

/// Одне поле "ВОС / посада / курс" (02 §3) — не org-scoped, як і `equipment_vos_hint`. Потрібен
/// ≥ 2 сторінкам (`pages::training_form`, `pages::import` — обидві через `widgets::group_grid`)
/// — тому тут, а не в `pages/<p>/server.rs` (07 §3.5).
#[server(SearchVosPositionCourse, "/api")]
pub async fn search_vos_position_course(
    query: String,
) -> Result<Vec<VosPositionCourseHint>, ServerFnError> {
    use crate::backend::repo;

    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::groups::search_vos_position_course(&db, &query)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Майданчики навчання обраної частини (02 §1 колонка 5). Той самий "≥ 2 сторінки" привід.
#[server(GetTrainingSites, "/api")]
pub async fn get_training_sites(org_id: i32) -> Result<Vec<TrainingSiteOption>, ServerFnError> {
    use crate::backend::repo;

    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::groups::training_site_options(&db, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
