use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        // btree_gist — потрібен для EXCLUDE USING gist по (bigint/text, daterange) у subordination:
        // без нього GiST-індекс не вміє порівнювати звичайні (не геометричні) типи на рівність.
        db.execute_unprepared("CREATE EXTENSION IF NOT EXISTS btree_gist")
            .await?;
        // pg_trgm — нечіткий пошук (similarity) по alias.norm для підказок у формі введення (02).
        db.execute_unprepared("CREATE EXTENSION IF NOT EXISTS pg_trgm")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("DROP EXTENSION IF EXISTS pg_trgm")
            .await?;
        db.execute_unprepared("DROP EXTENSION IF EXISTS btree_gist")
            .await?;
        Ok(())
    }
}
