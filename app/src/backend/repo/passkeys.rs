use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult};

#[derive(FromQueryResult)]
struct CredentialRow {
    id: i32,
    credential_id: Vec<u8>,
    public_key: Vec<u8>,
    sign_count: i64,
    name: Option<String>,
}

pub struct PasskeyCredential {
    pub id: i32,
    pub credential_id: Vec<u8>,
    pub public_key: Vec<u8>,
    pub sign_count: u64,
    pub name: Option<String>,
}

pub async fn credentials_for_user(
    db: &DatabaseConnection,
    user_id: i32,
) -> Result<Vec<PasskeyCredential>, DbErr> {
    let rows = CredentialRow::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT id, credential_id, public_key, sign_count, name \
         FROM passkey_credential WHERE user_id = $1 ORDER BY created_at",
        [user_id.into()],
    ))
    .all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| PasskeyCredential {
            id: r.id,
            credential_id: r.credential_id,
            public_key: r.public_key,
            sign_count: r.sign_count as u64,
            name: r.name,
        })
        .collect())
}

pub async fn store_credential(
    db: &impl ConnectionTrait,
    user_id: i32,
    credential_id: &[u8],
    public_key: &[u8],
    name: Option<&str>,
) -> Result<i32, DbErr> {
    #[derive(FromQueryResult)]
    struct Id {
        id: i32,
    }

    let row = Id::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "INSERT INTO passkey_credential (user_id, credential_id, public_key, name) \
         VALUES ($1, $2, $3, $4) RETURNING id",
        [
            user_id.into(),
            credential_id.into(),
            public_key.into(),
            name.map(|s| sea_orm::Value::String(Some(Box::new(s.to_string()))))
                .unwrap_or(sea_orm::Value::String(None)),
        ],
    ))
    .one(db)
    .await?
    .ok_or(DbErr::Custom("insert returned nothing".to_string()))?;

    Ok(row.id)
}

pub async fn find_credential_by_id(
    db: &DatabaseConnection,
    credential_id: &[u8],
) -> Result<Option<(i32, PasskeyCredential)>, DbErr> {
    #[derive(FromQueryResult)]
    struct WithUserId {
        user_id: i32,
        id: i32,
        credential_id: Vec<u8>,
        public_key: Vec<u8>,
        sign_count: i64,
        name: Option<String>,
    }

    let row = WithUserId::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT user_id, id, credential_id, public_key, sign_count, name \
         FROM passkey_credential WHERE credential_id = $1",
        [credential_id.into()],
    ))
    .one(db)
    .await?;

    Ok(row.map(|r| {
        (
            r.user_id,
            PasskeyCredential {
                id: r.id,
                credential_id: r.credential_id,
                public_key: r.public_key,
                sign_count: r.sign_count as u64,
                name: r.name,
            },
        )
    }))
}

pub async fn update_sign_count(
    db: &impl ConnectionTrait,
    credential_db_id: i32,
    new_count: u64,
) -> Result<(), DbErr> {
    db.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "UPDATE passkey_credential SET sign_count = $1 WHERE id = $2",
        [(new_count as i64).into(), credential_db_id.into()],
    ))
    .await?;
    Ok(())
}

pub async fn delete_credential(
    db: &impl ConnectionTrait,
    user_id: i32,
    credential_db_id: i32,
) -> Result<bool, DbErr> {
    let res = db
        .execute(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "DELETE FROM passkey_credential WHERE id = $1 AND user_id = $2",
            [credential_db_id.into(), user_id.into()],
        ))
        .await?;
    Ok(res.rows_affected() > 0)
}
