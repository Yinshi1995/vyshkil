use sea_orm::{ConnectionTrait, DbErr, FromQueryResult, Statement};

// ---------------------------------------------------------------------------
// City
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, FromQueryResult)]
pub struct CityRow {
    pub id: i32,
    pub name: String,
}

pub async fn list_cities(db: &impl ConnectionTrait) -> Result<Vec<CityRow>, DbErr> {
    CityRow::find_by_statement(Statement::from_string(
        db.get_database_backend(),
        "SELECT id, name FROM city ORDER BY name".to_owned(),
    ))
    .all(db)
    .await
}

pub async fn search_cities(
    db: &impl ConnectionTrait,
    q: &str,
    limit: i64,
) -> Result<Vec<CityRow>, DbErr> {
    if q.is_empty() {
        return list_cities(db).await;
    }
    CityRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, name FROM city WHERE name ILIKE '%' || $1 || '%' ORDER BY similarity(name, $1) DESC LIMIT $2",
        [q.into(), limit.into()],
    ))
    .all(db)
    .await
}

pub async fn create_city(db: &impl ConnectionTrait, name: &str) -> Result<i32, DbErr> {
    #[derive(FromQueryResult)]
    struct Id { id: i32 }
    let row = Id::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO city (name) VALUES ($1) RETURNING id",
        [name.into()],
    ))
    .one(db)
    .await?
    .ok_or_else(|| DbErr::Custom("no id returned".into()))?;
    Ok(row.id)
}

pub async fn update_city(db: &impl ConnectionTrait, id: i32, name: &str) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE city SET name = $1 WHERE id = $2",
        [name.into(), id.into()],
    ))
    .await?;
    Ok(())
}

pub async fn delete_city(db: &impl ConnectionTrait, id: i32) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "DELETE FROM city WHERE id = $1",
        [id.into()],
    ))
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Training Venue
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, FromQueryResult)]
pub struct VenueRow {
    pub id: i32,
    pub kind: String,
    pub name: String,
    pub short_name: Option<String>,
    pub military_number: Option<String>,
    pub city_id: i32,
    pub city_name: String,
    pub org_id: Option<i32>,
    pub is_active: bool,
}

pub async fn list_venues(db: &impl ConnectionTrait) -> Result<Vec<VenueRow>, DbErr> {
    VenueRow::find_by_statement(Statement::from_string(
        db.get_database_backend(),
        "SELECT tv.id, tv.kind, tv.name, tv.short_name, tv.military_number, \
                tv.city_id, c.name AS city_name, tv.org_id, tv.is_active \
         FROM training_venue tv JOIN city c ON c.id = tv.city_id \
         ORDER BY tv.kind, tv.name"
            .to_owned(),
    ))
    .all(db)
    .await
}

#[derive(Debug, Clone, serde::Serialize, FromQueryResult)]
pub struct VenueSearchRow {
    pub id: i32,
    pub kind: String,
    pub name: String,
    pub city_id: i32,
    pub city_name: String,
}

pub async fn search_venues(
    db: &impl ConnectionTrait,
    q: &str,
    limit: i64,
) -> Result<Vec<VenueSearchRow>, DbErr> {
    if q.is_empty() {
        return VenueSearchRow::find_by_statement(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT tv.id, tv.kind, tv.name, tv.city_id, c.name AS city_name \
             FROM training_venue tv JOIN city c ON c.id = tv.city_id \
             WHERE tv.is_active ORDER BY tv.name LIMIT $1",
            [limit.into()],
        ))
        .all(db)
        .await;
    }
    VenueSearchRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT tv.id, tv.kind, tv.name, tv.city_id, c.name AS city_name \
         FROM training_venue tv JOIN city c ON c.id = tv.city_id \
         WHERE tv.is_active AND (tv.name ILIKE '%' || $1 || '%' OR c.name ILIKE '%' || $1 || '%') \
         ORDER BY similarity(tv.name, $1) DESC LIMIT $2",
        [q.into(), limit.into()],
    ))
    .all(db)
    .await
}

pub async fn create_venue(
    db: &impl ConnectionTrait,
    kind: &str,
    name: &str,
    short_name: Option<&str>,
    military_number: Option<&str>,
    city_id: i32,
    org_id: Option<i32>,
) -> Result<i32, DbErr> {
    #[derive(FromQueryResult)]
    struct Id { id: i32 }
    let row = Id::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO training_venue (kind, name, short_name, military_number, city_id, org_id) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
        [
            kind.into(),
            name.into(),
            short_name.map(|s| s.into()).unwrap_or(sea_orm::Value::String(None)),
            military_number.map(|s| s.into()).unwrap_or(sea_orm::Value::String(None)),
            city_id.into(),
            org_id.map(|i| i.into()).unwrap_or(sea_orm::Value::Int(None)),
        ],
    ))
    .one(db)
    .await?
    .ok_or_else(|| DbErr::Custom("no id returned".into()))?;
    Ok(row.id)
}

pub async fn update_venue(
    db: &impl ConnectionTrait,
    id: i32,
    kind: &str,
    name: &str,
    short_name: Option<&str>,
    military_number: Option<&str>,
    city_id: i32,
    org_id: Option<i32>,
) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE training_venue SET kind = $1, name = $2, short_name = $3, \
         military_number = $4, city_id = $5, org_id = $6, updated_at = now() WHERE id = $7",
        [
            kind.into(),
            name.into(),
            short_name.map(|s| s.into()).unwrap_or(sea_orm::Value::String(None)),
            military_number.map(|s| s.into()).unwrap_or(sea_orm::Value::String(None)),
            city_id.into(),
            org_id.map(|i| i.into()).unwrap_or(sea_orm::Value::Int(None)),
            id.into(),
        ],
    ))
    .await?;
    Ok(())
}

pub async fn delete_venue(db: &impl ConnectionTrait, id: i32) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "DELETE FROM training_venue WHERE id = $1",
        [id.into()],
    ))
    .await?;
    Ok(())
}

pub async fn venue_city_id(db: &impl ConnectionTrait, venue_id: i32) -> Result<Option<i32>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row { city_id: i32 }
    Ok(Row::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT city_id FROM training_venue WHERE id = $1",
        [venue_id.into()],
    ))
    .one(db)
    .await?
    .map(|r| r.city_id))
}

pub async fn org_current_city_id(db: &impl ConnectionTrait, org_id: i32) -> Result<Option<i32>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row { current_city_id: Option<i32> }
    Ok(Row::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT current_city_id FROM org WHERE id = $1",
        [org_id.into()],
    ))
    .one(db)
    .await?
    .and_then(|r| r.current_city_id))
}
