pub use sea_orm_migration::prelude::*;

mod m20240115_000001_create_unit_table;
mod m20240115_000002_create_personnel_table;
mod m20240115_000003_create_exercise_table;
mod m20240115_000004_create_training_session_table;
mod m20240115_000005_create_metric_result_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        // Порядок важливий: FK-и посилаються на попередні таблиці, тому unit -> personnel/exercise -> training_session -> metric_result.
        vec![
            Box::new(m20240115_000001_create_unit_table::Migration),
            Box::new(m20240115_000002_create_personnel_table::Migration),
            Box::new(m20240115_000003_create_exercise_table::Migration),
            Box::new(m20240115_000004_create_training_session_table::Migration),
            Box::new(m20240115_000005_create_metric_result_table::Migration),
        ]
    }
}
