use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Org::Table)
                    .if_not_exists()
                    .col(pk_auto(Org::Id))
                    .col(
                        ColumnDef::new(Org::Kind)
                            .string()
                            .not_null()
                            .check(Expr::col(Org::Kind).is_in([
                                "military_unit",
                                "command",
                                "virtual_group",
                                "subunit",
                                "edu_institution",
                                "company",
                                "ngo",
                                "foreign_state",
                                "other",
                            ])),
                    )
                    // echelon — вільний текст, а не CHECK: новий рівень ієрархії (рота/батальйон/.../ксв)
                    // не повинен вимагати міграції (docs/spec/01-domain-model.md §1).
                    .col(string_null(Org::Echelon))
                    .col(
                        ColumnDef::new(Org::NumberKind)
                            .string()
                            .null()
                            .check(Expr::col(Org::NumberKind).is_in(["A", "T", "NGU"])),
                    )
                    .col(string_null(Org::Number))
                    .col(string(Org::ShortName))
                    .col(string_null(Org::FullName))
                    .col(string_null(Org::Branch))
                    .col(string_null(Org::Country))
                    .col(boolean(Org::IsActive).default(true))
                    .col(
                        ColumnDef::new(Org::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Org::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Org::DeletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Часткова унікальність номера лише серед military_unit (01 §1): інші kind можуть
        // випадково мати той самий текстовий "номер" без реального конфлікту.
        // Два окремі індекси (а не один на (number_kind, number)): Postgres рахує кожен NULL
        // окремим значенням у unique-індексі, тому "number_kind IS NULL" довелось би розрізняти
        // окремо — інакше два ВЧ з number_kind=NULL і однаковим number не зловились би.
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE UNIQUE INDEX idx_org_military_unit_number ON org (number_kind, number) \
                 WHERE kind = 'military_unit' AND number_kind IS NOT NULL AND number IS NOT NULL",
            )
            .await?;
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE UNIQUE INDEX idx_org_military_unit_number_no_kind ON org (number) \
                 WHERE kind = 'military_unit' AND number_kind IS NULL AND number IS NOT NULL",
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Org::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Org {
    Table,
    Id,
    Kind,
    Echelon,
    NumberKind,
    Number,
    ShortName,
    FullName,
    Branch,
    Country,
    IsActive,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}
