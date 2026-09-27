use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

// Таблиці, які редагуються напряму (не системно-похідні) — під аудит, за 01-domain-model.md §5
// ("generic тригер на всі доменні таблиці"). Виняток — лише subordination_closure: вона повністю
// TRUNCATE+INSERT перебудовується при кожній зміні subordination, аудит там був би шумом поверх шуму.
const AUDITED_TABLES: [&str; 6] = [
    "org",
    "org_name_history",
    "training_site",
    "subordination",
    "org_status",
    "alias",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AuditLog::Table)
                    .if_not_exists()
                    .col(pk_auto(AuditLog::Id))
                    .col(string(AuditLog::TableName))
                    .col(integer(AuditLog::RowId))
                    .col(
                        ColumnDef::new(AuditLog::Action)
                            .string()
                            .not_null()
                            .check(Expr::col(AuditLog::Action).is_in([
                                "insert", "update", "delete",
                            ])),
                    )
                    .col(ColumnDef::new(AuditLog::Before).json_binary().null())
                    .col(ColumnDef::new(AuditLog::After).json_binary().null())
                    // Не ПІБ — ідентифікатор актора (організація+роль), див. server::policy.
                    .col(string_null(AuditLog::Actor))
                    .col(
                        ColumnDef::new(AuditLog::At)
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
                    .name("idx-audit_log-table_name-row_id")
                    .table(AuditLog::Table)
                    .col(AuditLog::TableName)
                    .col(AuditLog::RowId)
                    .to_owned(),
            )
            .await?;

        let db = manager.get_connection();

        // current_setting('app.actor', true) читає сесійну змінну, яку сервер виставляє
        // `SET LOCAL app.actor = '<org>:<role>'` перед кожним запитом (server::policy) —
        // так тригер знає актора без передачі його через кожен INSERT/UPDATE вручну.
        db.execute_unprepared(
            "CREATE OR REPLACE FUNCTION audit_log_trigger() RETURNS trigger AS $$
            DECLARE
                v_actor text;
            BEGIN
                v_actor := current_setting('app.actor', true);
                IF TG_OP = 'INSERT' THEN
                    INSERT INTO audit_log(table_name, row_id, action, before, after, actor, at)
                    VALUES (TG_TABLE_NAME, NEW.id, 'insert', NULL, to_jsonb(NEW), v_actor, now());
                    RETURN NEW;
                ELSIF TG_OP = 'UPDATE' THEN
                    INSERT INTO audit_log(table_name, row_id, action, before, after, actor, at)
                    VALUES (TG_TABLE_NAME, NEW.id, 'update', to_jsonb(OLD), to_jsonb(NEW), v_actor, now());
                    RETURN NEW;
                ELSIF TG_OP = 'DELETE' THEN
                    INSERT INTO audit_log(table_name, row_id, action, before, after, actor, at)
                    VALUES (TG_TABLE_NAME, OLD.id, 'delete', to_jsonb(OLD), NULL, v_actor, now());
                    RETURN OLD;
                END IF;
                RETURN NULL;
            END;
            $$ LANGUAGE plpgsql",
        )
        .await?;

        for table in AUDITED_TABLES {
            db.execute_unprepared(&format!(
                "CREATE TRIGGER trg_audit_{table} AFTER INSERT OR UPDATE OR DELETE ON {table} \
                 FOR EACH ROW EXECUTE FUNCTION audit_log_trigger()"
            ))
            .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for table in AUDITED_TABLES {
            db.execute_unprepared(&format!("DROP TRIGGER IF EXISTS trg_audit_{table} ON {table}"))
                .await?;
        }
        db.execute_unprepared("DROP FUNCTION IF EXISTS audit_log_trigger()")
            .await?;
        manager
            .drop_table(Table::drop().table(AuditLog::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum AuditLog {
    Table,
    Id,
    TableName,
    RowId,
    Action,
    Before,
    After,
    Actor,
    At,
}
