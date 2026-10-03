use leptos::prelude::*;

#[server(RequestAccountFn, "/api")]
pub async fn request_account(contact: String, unit: String, message: String) -> Result<(), ServerFnError> {
    use crate::backend::db::connection;
    use crate::backend::repo;
    use sea_orm::{ConnectionTrait, FromQueryResult, Statement};

    if contact.trim().is_empty() {
        return Err(ServerFnError::new("Вкажіть контактні дані"));
    }

    let db = connection();

    #[derive(FromQueryResult)]
    struct AdminOrg { org_id: i32 }
    let admin = AdminOrg::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT ur.org_id FROM user_role ur \
         JOIN user_account ua ON ua.id = ur.user_id \
         WHERE ur.role = 'admin' LIMIT 1",
        [],
    ))
    .one(&db)
    .await
    .map_err(|e| ServerFnError::new(format!("{e}")))?;

    let org_id = admin.map(|r| r.org_id).unwrap_or(1);

    let body = format!(
        "Контакт: {}\nЧастина: {}\nПовідомлення: {}",
        contact.trim(),
        if unit.trim().is_empty() { "—" } else { unit.trim() },
        if message.trim().is_empty() { "—" } else { message.trim() },
    );

    repo::notifications::insert(
        &db, org_id, "system",
        "Запит на створення облікового запису",
        Some(&body), Some("/settings"),
    ).await.map_err(|e| ServerFnError::new(format!("{e}")))?;

    Ok(())
}
