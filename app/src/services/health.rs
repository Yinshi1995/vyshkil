use leptos::prelude::*;

// #[server] генерує однакову сигнатуру для обох таргетів: на клієнті це виклик по мережі,
// на сервері (під feature "ssr") — реальне тіло, що бере DatabaseConnection з контексту Leptos-роуту.
#[server(HealthCheck, "/api")]
pub async fn health_check() -> Result<String, ServerFnError> {
    let db = expect_context::<sea_orm::DatabaseConnection>();
    db.ping().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok("з'єднано".to_string())
}
