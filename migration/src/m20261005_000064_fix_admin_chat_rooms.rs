use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        // Admin (user_id=1) should only be in general room (id=1) and
        // their primary org room УВС Південь (id=2, org_id=1).
        // Remove from all other org-specific rooms to avoid
        // appearing as a member of subordinate orgs.
        db.execute_unprepared(
            "DELETE FROM chat_room_member \
             WHERE user_id = 1 \
               AND room_id NOT IN (1, 2)"
        ).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
