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
mod m20260929_000034_create_staffing_tables;
mod m20260929_000035_attach_audit_triggers_staffing;
mod m20260929_000036_refine_subordination_dates_from_kontrolka;
mod m20260929_000037_create_generated_document_table;
mod m20260930_000038_widen_generated_document_kind;
mod m20260930_000039_create_reported_group_table;
mod m20260930_000040_create_discrepancy_table;
mod m20260930_000041_attach_audit_trigger_discrepancy;
mod m20260930_000042_create_outbox_table;
mod m20261001_000043_create_notification_table;
mod m20261001_000044_widen_generated_document_kind_d3_d6;
mod m20261001_000045_add_search_indexes;
mod m20261001_000046_create_user_account_table;
mod m20261001_000047_seed_admin_account;
mod m20261002_000048_create_passkey_credential_table;
mod m20261002_000049_create_whatsapp_notification_tables;
mod m20261002_000050_seed_comprehensive;
mod m20261002_000051_add_must_change_password;
mod m20261002_000052_add_group_event_notification_type;
mod m20261004_000053_add_user_profile_fields;
mod m20261004_000054_add_user_name_rank_avatar;
mod m20261004_000055_add_delta_nick;
mod m20261004_000056_add_event_created_by;
mod m20261004_000057_create_chat_tables;
mod m20261004_000058_add_performance_indexes;
mod m20261005_000059_add_whatsapp_group_kind;
mod m20261005_000060_dedup_training_group;
mod m20261005_000062_add_can_see_org_names;
mod m20261005_000063_add_sender_org_index;
mod m20261005_000064_fix_admin_chat_rooms;
mod m20261005_000065_create_training_venue_model;
mod m20261005_000066_add_org_masked_label;

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
            // Етап 5: staffing_snapshot/staffing_metric (01 §4) — потрібні КВід/ІВС, яких не було
            // в жодному попередньому етапі.
            Box::new(m20260929_000034_create_staffing_tables::Migration),
            Box::new(m20260929_000035_attach_audit_triggers_staffing::Migration),
            // Етап 6: реальні дати переходу 17 АК → 7 КШР з Контролька (замість умовної
            // 01.08.2026 у dev-сіді).
            Box::new(m20260929_000036_refine_subordination_dates_from_kontrolka::Migration),
            // Етап 7: generated_document (05 §вступ) — мінімум під D1.
            Box::new(m20260929_000037_create_generated_document_table::Migration),
            // Етап 7, D2: розширити kind на 'd2'.
            Box::new(m20260930_000038_widen_generated_document_kind::Migration),
            // Етап 8, зріз 1 (04): reported_group (незмінний знімок подання) + discrepancy
            // (горизонтальна звірка) — .claude/decisions/etap8-horizontal-reconciliation-first-slice.md.
            Box::new(m20260930_000039_create_reported_group_table::Migration),
            Box::new(m20260930_000040_create_discrepancy_table::Migration),
            Box::new(m20260930_000041_attach_audit_trigger_discrepancy::Migration),
            // Брокер (09-messaging.md, Фаза 1): транзакційний outbox — .claude/decisions/
            // broker-nats-jetstream.md.
            Box::new(m20260930_000042_create_outbox_table::Migration),
            // Етап 8, зріз 4 (04 §5): внутрішні сповіщення (дзвіночок у шапці).
            Box::new(m20261001_000043_create_notification_table::Migration),
            // Етап 9 (D5-D6): розширити CHECK на generated_document.kind (d3-d6, також
            // виправляє баг — d3/d4 insert раніше мовчки падав на CHECK).
            Box::new(m20261001_000044_widen_generated_document_kind_d3_d6::Migration),
            Box::new(m20261001_000045_add_search_indexes::Migration),
            // Етап 10a (12-auth.md): автентифікація логін/пароль.
            Box::new(m20261001_000046_create_user_account_table::Migration),
            Box::new(m20261001_000047_seed_admin_account::Migration),
            // Étap 10b (12-auth.md §1.3): FIDO2/WebAuthn passkey credentials.
            Box::new(m20261002_000048_create_passkey_credential_table::Migration),
            // WhatsApp-сповіщення (09-messaging.md §3.8): адреси доставки, типи, підписки, групи.
            Box::new(m20261002_000049_create_whatsapp_notification_tables::Migration),
            // Комплексний dev-сід: користувачі, групи, воронка, подання, розбіжності, staffing, WhatsApp.
            Box::new(m20261002_000050_seed_comprehensive::Migration),
            // must_change_password для flow тимчасового пароля.
            Box::new(m20261002_000051_add_must_change_password::Migration),
            // Новий тип нотифікації для подій груп підготовки.
            Box::new(m20261002_000052_add_group_event_notification_type::Migration),
            Box::new(m20261004_000053_add_user_profile_fields::Migration),
            Box::new(m20261004_000054_add_user_name_rank_avatar::Migration),
            Box::new(m20261004_000055_add_delta_nick::Migration),
            Box::new(m20261004_000056_add_event_created_by::Migration),
            Box::new(m20261004_000057_create_chat_tables::Migration),
            Box::new(m20261004_000058_add_performance_indexes::Migration),
            Box::new(m20261005_000059_add_whatsapp_group_kind::Migration),
            Box::new(m20261005_000060_dedup_training_group::Migration),
            Box::new(m20261005_000062_add_can_see_org_names::Migration),
            Box::new(m20261005_000063_add_sender_org_index::Migration),
            Box::new(m20261005_000064_fix_admin_chat_rooms::Migration),
            // Структуровані місця навчання: city + training_venue замість training_site.
            Box::new(m20261005_000065_create_training_venue_model::Migration),
            // Generated column org.masked_label — маскований лейбл (А-номер або short_name),
            // обчислюється Postgres, індексований pg_trgm — замість окремого запиту org_number_labels.
            Box::new(m20261005_000066_add_org_masked_label::Migration),
        ]
    }
}
