use sea_orm_migration::{prelude::*, schema::*};

use crate::m20260928_000025_create_training_group_table::TrainingGroup;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Колонки training_group, свідомо відкладені міграцією 000025 ("не потрібні для критерію
// Етапу 3") -- тепер потрібні: сітка форми (02 §1, колонки 4 і 9) їх показує.
// group_equipment (зв'язок з довідником equipment) лишається відкладеним -- сітка Етапу 4
// зберігає ОВТ як вільний текст; розпізнавання через equipment_vos лишається підказкою при вводі
// ВОС (вже є, dictionaries), не окремим полем форми.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(TrainingGroup::Table)
                    .add_column(text_null(TrainingGroup::EquipmentText))
                    .add_column(string_null(TrainingGroup::BasisDocNumber))
                    .add_column(ColumnDef::new(TrainingGroup::BasisDocDate).date().null())
                    // Для adaptation: "з НЦ (ЦПП)" / "перепризначення" (01 §3).
                    .add_column(string_null(TrainingGroup::InflowSource))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(TrainingGroup::Table)
                    .drop_column(TrainingGroup::EquipmentText)
                    .drop_column(TrainingGroup::BasisDocNumber)
                    .drop_column(TrainingGroup::BasisDocDate)
                    .drop_column(TrainingGroup::InflowSource)
                    .to_owned(),
            )
            .await
    }
}
