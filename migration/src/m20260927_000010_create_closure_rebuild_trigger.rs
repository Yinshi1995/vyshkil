use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Перебудовується ЦІЛКОМ в одній транзакції при будь-якій зміні subordination (01 §1).
        // Рекурсивний CTE тут — не порушення "не рекурсивні CTE в гарячих шляхах" (server/CLAUDE.md):
        // це періодичний rebuild (тригер на subordination, не на кожен читання), гарячий шлях читання
        // йде через subordination_closure напряму (server::policy::can_view_org і майбутні звіти).
        // Період "предок на два кроки" = перетин періодів обох ланок ланцюга — тому GREATEST/LEAST.
        // depth < 20 — захист від нескінченної рекурсії при помилкових циклічних даних.
        db.execute_unprepared(
            "CREATE OR REPLACE FUNCTION rebuild_subordination_closure() RETURNS trigger AS $$
            BEGIN
                TRUNCATE subordination_closure;
                INSERT INTO subordination_closure (ancestor_id, descendant_id, axis, depth, valid_from, valid_to)
                WITH RECURSIVE closure AS (
                    SELECT
                        child_org_id AS descendant_id,
                        parent_org_id AS ancestor_id,
                        axis,
                        1 AS depth,
                        valid_from,
                        valid_to
                    FROM subordination
                    UNION ALL
                    SELECT
                        c.descendant_id,
                        s.parent_org_id,
                        s.axis,
                        c.depth + 1,
                        GREATEST(c.valid_from, s.valid_from),
                        CASE
                            WHEN c.valid_to IS NULL THEN s.valid_to
                            WHEN s.valid_to IS NULL THEN c.valid_to
                            ELSE LEAST(c.valid_to, s.valid_to)
                        END
                    FROM closure c
                    JOIN subordination s ON s.child_org_id = c.ancestor_id AND s.axis = c.axis
                    WHERE c.depth < 20
                      AND GREATEST(c.valid_from, s.valid_from) < COALESCE(
                            CASE
                                WHEN c.valid_to IS NULL THEN s.valid_to
                                WHEN s.valid_to IS NULL THEN c.valid_to
                                ELSE LEAST(c.valid_to, s.valid_to)
                            END,
                            'infinity'::date)
                )
                SELECT ancestor_id, descendant_id, axis, depth, valid_from, valid_to FROM closure;
                RETURN NULL;
            END;
            $$ LANGUAGE plpgsql",
        )
        .await?;

        db.execute_unprepared(
            "CREATE TRIGGER trg_rebuild_subordination_closure \
             AFTER INSERT OR UPDATE OR DELETE ON subordination \
             FOR EACH STATEMENT EXECUTE FUNCTION rebuild_subordination_closure()",
        )
        .await
        .map(|_| ())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            "DROP TRIGGER IF EXISTS trg_rebuild_subordination_closure ON subordination",
        )
        .await?;
        db.execute_unprepared("DROP FUNCTION IF EXISTS rebuild_subordination_closure()")
            .await
            .map(|_| ())
    }
}
