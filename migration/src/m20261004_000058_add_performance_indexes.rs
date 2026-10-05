use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20261004_000058_add_performance_indexes"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // ── Session auth: index for expired session cleanup ──
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_user_session_expires
             ON user_session (expires_at)"
        ).await?;

        // ── Chat: room list for a user (reverse lookup member→rooms) ──
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_chat_room_member_user_id
             ON chat_room_member (user_id)"
        ).await?;

        // Chat: unread count + last message subqueries (partial: skip deleted)
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_chat_message_room_created_alive
             ON chat_message (room_id, created_at DESC) WHERE deleted_at IS NULL"
        ).await?;

        // ── Group events: SUM aggregation by (group_id, event_type) — hottest query ──
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_group_event_group_type
             ON group_event (group_id, event_type)"
        ).await?;

        // Group event: covering index for count aggregation
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_group_event_group_type_count
             ON group_event (group_id, event_type) INCLUDE (count)"
        ).await?;

        // ── Training groups: filter by kind (D5 reports) ──
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_training_group_kind
             ON training_group (training_kind_id)"
        ).await?;

        // Training groups: by site (join in reports)
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_training_group_site
             ON training_group (site_id)"
        ).await?;

        // ── Subordination closure: descendant lookup (org hierarchy traversal) ──
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_sub_closure_descendant_depth
             ON subordination_closure (descendant_id, depth)"
        ).await?;

        // Subordination closure: ancestor lookup
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_sub_closure_ancestor_depth
             ON subordination_closure (ancestor_id, depth)"
        ).await?;

        // ── Org: active orgs fast lookup ──
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_org_active
             ON org (id) WHERE deleted_at IS NULL"
        ).await?;

        // ── Reported groups: by sender org + submission ──
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_reported_group_sender_org
             ON reported_group (sender_org_id)"
        ).await?;

        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_reported_group_submission
             ON reported_group (submission_id)"
        ).await?;

        // ── Discrepancy: by org + open status (dashboard) ──
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_discrepancy_org_open
             ON discrepancy (org_id) WHERE status = 'open'"
        ).await?;

        // ── Submission: by org + status ──
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_submission_org_status
             ON submission (reporting_org_id, status)"
        ).await?;

        // ── Audit log: time-based queries ──
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_audit_log_at
             ON audit_log (at DESC)"
        ).await?;

        // ── Notification: unread per org (bell icon badge) ──
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_notification_org_unread
             ON notification (org_id, created_at DESC) WHERE is_read = false"
        ).await?;

        // ── User role: org lookup (who has access to org X) ──
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_user_role_org_id
             ON user_role (org_id)"
        ).await?;

        // ── Staffing: latest snapshot per org+category ──
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_staffing_snapshot_latest
             ON staffing_snapshot (org_id, category, as_of DESC)"
        ).await?;

        // ── Generated document: latest by kind ──
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_generated_document_kind_date
             ON generated_document (kind, as_of_date DESC)"
        ).await?;

        // ── pg_trgm: fast text search on org short_name ──
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_org_short_name_trgm
             ON org USING gin (short_name gin_trgm_ops)"
        ).await?;

        // ── Analyze all tables to update planner statistics ──
        db.execute_unprepared("ANALYZE").await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        let indexes = [
            "idx_user_session_expires",
            "idx_chat_room_member_user_id",
            "idx_chat_message_room_created_alive",
            "idx_group_event_group_type",
            "idx_group_event_group_type_count",
            "idx_training_group_kind",
            "idx_training_group_site",
            "idx_sub_closure_descendant_depth",
            "idx_sub_closure_ancestor_depth",
            "idx_org_active",
            "idx_reported_group_sender_org",
            "idx_reported_group_submission",
            "idx_discrepancy_org_open",
            "idx_submission_org_status",
            "idx_audit_log_at",
            "idx_notification_org_unread",
            "idx_user_role_org_id",
            "idx_staffing_snapshot_latest",
            "idx_generated_document_kind_date",
            "idx_org_short_name_trgm",
        ];

        for idx in &indexes {
            db.execute_unprepared(&format!("DROP INDEX IF EXISTS {idx}")).await?;
        }

        Ok(())
    }
}
