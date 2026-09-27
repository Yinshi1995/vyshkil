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
                    .table(TrainingSite::Table)
                    .if_not_exists()
                    .col(pk_auto(TrainingSite::Id))
                    .col(integer(TrainingSite::OrgId))
                    // locality = null означає "навчання на базі самої ВЧ" (01 §1), а не пропуск даних.
                    .col(string_null(TrainingSite::Locality))
                    .col(text_null(TrainingSite::Note))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-training_site-org_id")
                            .from(TrainingSite::Table, TrainingSite::OrgId)
                            .to(Org::Table, Org::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        // Unique (org_id, locality) — з урахуванням NULL: Postgres дозволяє кілька NULL у unique-індексі,
        // тому для "locality IS NULL" (навчання на базі самої ВЧ) додатково обмежуємо частковим індексом.
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE UNIQUE INDEX idx_training_site_org_locality \
                 ON training_site (org_id, locality) WHERE locality IS NOT NULL",
            )
            .await?;
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE UNIQUE INDEX idx_training_site_org_no_locality \
                 ON training_site (org_id) WHERE locality IS NULL",
            )
            .await
            .map(|_| ())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TrainingSite::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum TrainingSite {
    Table,
    Id,
    OrgId,
    Locality,
    Note,
}
