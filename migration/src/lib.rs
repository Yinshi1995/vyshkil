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
mod m20260928_000012_create_training_kind_table;
mod m20260928_000013_create_training_direction_table;
mod m20260928_000014_create_bzvp_program_table;
mod m20260928_000015_create_vos_table;
mod m20260928_000016_create_position_table;
mod m20260928_000017_create_vos_position_table;
mod m20260928_000018_create_equipment_table;
mod m20260928_000019_create_equipment_vos_table;
mod m20260928_000020_create_course_table;
mod m20260928_000021_create_attrition_reason_table;
mod m20260928_000022_attach_audit_triggers_stage2;
mod m20260928_000023_seed_stage2_dictionaries;
mod m20260928_000024_seed_full_vos_dictionary;
mod m20260928_000025_create_training_group_table;
mod m20260928_000026_create_group_event_table;
mod m20260928_000027_attach_audit_triggers_stage3;
mod m20260928_000028_create_submission_table;
mod m20260928_000029_add_submission_id_to_group_event;
mod m20260928_000030_attach_audit_trigger_submission;
mod m20260928_000031_extend_training_group_for_stage4;
mod m20260928_000032_create_group_composition_table;
mod m20260928_000033_attach_audit_trigger_group_composition;

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
            // Етап 2 (docs/spec/06-roadmap.md): довідники підготовки + словник спеціальностей.
            Box::new(m20260928_000012_create_training_kind_table::Migration),
            Box::new(m20260928_000013_create_training_direction_table::Migration),
            Box::new(m20260928_000014_create_bzvp_program_table::Migration),
            Box::new(m20260928_000015_create_vos_table::Migration),
            Box::new(m20260928_000016_create_position_table::Migration),
            Box::new(m20260928_000017_create_vos_position_table::Migration),
            Box::new(m20260928_000018_create_equipment_table::Migration),
            Box::new(m20260928_000019_create_equipment_vos_table::Migration),
            Box::new(m20260928_000020_create_course_table::Migration),
            Box::new(m20260928_000021_create_attrition_reason_table::Migration),
            Box::new(m20260928_000022_attach_audit_triggers_stage2::Migration),
            Box::new(m20260928_000023_seed_stage2_dictionaries::Migration),
            Box::new(m20260928_000024_seed_full_vos_dictionary::Migration),
            Box::new(m20260928_000025_create_training_group_table::Migration),
            Box::new(m20260928_000026_create_group_event_table::Migration),
            Box::new(m20260928_000027_attach_audit_triggers_stage3::Migration),
            // Етап 4: submission(status='draft') для автозбереження форми (02 §6).
            Box::new(m20260928_000028_create_submission_table::Migration),
            Box::new(m20260928_000029_add_submission_id_to_group_event::Migration),
            Box::new(m20260928_000030_attach_audit_trigger_submission::Migration),
            // Колонки/таблиці training_group відкладені Етапом 3, потрібні сітці Етапу 4.
            Box::new(m20260928_000031_extend_training_group_for_stage4::Migration),
            Box::new(m20260928_000032_create_group_composition_table::Migration),
            Box::new(m20260928_000033_attach_audit_trigger_group_composition::Migration),
        ]
    }
}
