use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Alias::Table)
                    .if_not_exists()
                    .col(pk_auto(Alias::Id))
                    .col(
                        ColumnDef::new(Alias::TargetType)
                            .string()
                            .not_null()
                            .check(Expr::col(Alias::TargetType).is_in([
                                "org",
                                "site",
                                "vos",
                                "position",
                                "equipment",
                                "course",
                                "attrition_reason",
                            ])),
                    )
                    // Без FK: target_id полiморфний (посилається на різні таблиці залежно від target_type).
                    .col(integer(Alias::TargetId))
                    .col(string(Alias::Raw))
                    .col(string(Alias::Norm))
                    .col(
                        ColumnDef::new(Alias::Source)
                            .string()
                            .not_null()
                            .check(Expr::col(Alias::Source).is_in(["seed", "manual", "learned"])),
                    )
                    .col(decimal_null(Alias::Confidence))
                    .col(integer(Alias::UsesCount).default(0))
                    // Не ПІБ — ідентифікатор організації-актора, яка створила синонім (немає авторизації осіб).
                    .col(string_null(Alias::CreatedBy))
                    .col(
                        ColumnDef::new(Alias::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-alias-target_type-target_id")
                    .table(Alias::Table)
                    .col(Alias::TargetType)
                    .col(Alias::TargetId)
                    .to_owned(),
            )
            .await?;

        // Нечіткий пошук для підказок (02 §3): "152нц" → "152 нц (А4896)" навіть з опечаткою.
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE INDEX idx_alias_norm_trgm ON alias USING gin (norm gin_trgm_ops)",
            )
            .await
            .map(|_| ())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Alias::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Alias {
    Table,
    Id,
    TargetType,
    TargetId,
    Raw,
    Norm,
    Source,
    Confidence,
    UsesCount,
    CreatedBy,
    CreatedAt,
}
