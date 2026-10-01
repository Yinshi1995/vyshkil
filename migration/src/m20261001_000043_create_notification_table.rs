use sea_orm_migration::{prelude::*, schema::*};

use crate::m20260927_000002_create_org_table::Org;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Notification::Table)
                    .if_not_exists()
                    .col(pk_auto(Notification::Id))
                    .col(integer(Notification::OrgId))
                    .col(string(Notification::Kind))
                    .col(string(Notification::Title))
                    .col(text_null(Notification::Body))
                    .col(string_null(Notification::Link))
                    .col(
                        ColumnDef::new(Notification::IsRead)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Notification::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-notification-org_id")
                            .from(Notification::Table, Notification::OrgId)
                            .to(Org::Table, Org::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-notification-org_id-is_read")
                    .table(Notification::Table)
                    .col(Notification::OrgId)
                    .col(Notification::IsRead)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Notification::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum Notification {
    Table,
    Id,
    OrgId,
    Kind,
    Title,
    Body,
    Link,
    IsRead,
    CreatedAt,
}
