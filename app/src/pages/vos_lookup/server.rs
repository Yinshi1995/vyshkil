use leptos::prelude::*;

use crate::types::dictionaries::EquipmentVosHint;

/// Підказка "ОВТ/сленг → ВОС" (02 §3: "вамп" → 218, "mavic" → 217, "fpv" → 219, "нрк" → 129,
/// "darts" → 216). Довідникові дані — не org-scoped, тому без `policy` (як і `services::orgs::
/// list_orgs`). Потрібна лише цій сторінці — тому тут, а не в `services/`.
#[server(GetEquipmentVosHint, "/api")]
pub async fn get_equipment_vos_hint(query: String) -> Result<Vec<EquipmentVosHint>, ServerFnError> {
    use crate::backend::repo;

    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::dictionaries::equipment_vos_hint(&db, &query)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
