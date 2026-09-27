pub use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        // Порожньо: попередня (вигадана) схема прибрана при переході на docs/spec.
        // Наступні міграції — Етап 1 за docs/spec/06-roadmap.md, префікс m20260927_….
        vec![]
    }
}
