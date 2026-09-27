pub use sea_orm_migration::prelude::*;

mod m20260927_000001_enable_extensions;
mod m20260927_000002_create_org_table;
mod m20260927_000003_create_org_name_history_table;
mod m20260927_000004_create_training_site_table;
mod m20260927_000005_create_subordination_table;
mod m20260927_000006_create_org_status_table;
mod m20260927_000007_create_subordination_closure_table;
mod m20260927_000008_create_alias_table;
mod m20260927_000009_create_audit_log_and_triggers;
mod m20260927_000010_create_closure_rebuild_trigger;
mod m20260927_000011_seed_dev_data;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        // Порядок — за FK-залежностями: org перша (усе інше на неї посилається),
        // subordination_closure після subordination, audit_log/тригери останніми
        // (вішаються на вже створені таблиці).
        vec![
            Box::new(m20260927_000001_enable_extensions::Migration),
            Box::new(m20260927_000002_create_org_table::Migration),
            Box::new(m20260927_000003_create_org_name_history_table::Migration),
            Box::new(m20260927_000004_create_training_site_table::Migration),
            Box::new(m20260927_000005_create_subordination_table::Migration),
            Box::new(m20260927_000006_create_org_status_table::Migration),
            Box::new(m20260927_000007_create_subordination_closure_table::Migration),
            Box::new(m20260927_000008_create_alias_table::Migration),
            Box::new(m20260927_000009_create_audit_log_and_triggers::Migration),
            Box::new(m20260927_000010_create_closure_rebuild_trigger::Migration),
            Box::new(m20260927_000011_seed_dev_data::Migration),
        ]
    }
}
