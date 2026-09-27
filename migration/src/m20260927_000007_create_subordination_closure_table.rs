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
                    .table(SubordinationClosure::Table)
                    .if_not_exists()
                    .col(pk_auto(SubordinationClosure::Id))
                    .col(integer(SubordinationClosure::AncestorId))
                    .col(integer(SubordinationClosure::DescendantId))
                    .col(
                        ColumnDef::new(SubordinationClosure::Axis)
                            .string()
                            .not_null()
                            .check(
                                Expr::col(SubordinationClosure::Axis)
                                    .is_in(["staff", "operational"]),
                            ),
                    )
                    .col(integer(SubordinationClosure::Depth))
                    .col(
                        ColumnDef::new(SubordinationClosure::ValidFrom)
                            .date()
                            .not_null(),
                    )
                    .col(ColumnDef::new(SubordinationClosure::ValidTo).date().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-subordination_closure-ancestor_id")
                            .from(SubordinationClosure::Table, SubordinationClosure::AncestorId)
                            .to(Org::Table, Org::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-subordination_closure-descendant_id")
                            .from(
                                SubordinationClosure::Table,
                                SubordinationClosure::DescendantId,
                            )
                            .to(Org::Table, Org::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Гарячий запит "усі підлеглі X за віссю A на дату D" (01 §1) — один btree-прохід
        // по (ancestor_id, axis) + один GiST-прохід по діапазону, разом як bitmap AND.
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-subordination_closure-ancestor_axis")
                    .table(SubordinationClosure::Table)
                    .col(SubordinationClosure::AncestorId)
                    .col(SubordinationClosure::Axis)
                    .to_owned(),
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                "CREATE INDEX idx_subordination_closure_valid_range \
                 ON subordination_closure USING gist (daterange(valid_from, valid_to, '[)'))",
            )
            .await
            .map(|_| ())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SubordinationClosure::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum SubordinationClosure {
    Table,
    Id,
    AncestorId,
    DescendantId,
    Axis,
    Depth,
    ValidFrom,
    ValidTo,
}
