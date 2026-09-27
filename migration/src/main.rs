use sea_orm_migration::prelude::*;

// Дозволяє керувати міграціями напряму через `cargo run -p migration -- up/down/status`
// без встановлення sea-orm-cli як окремого інструмента на CI/сервері.
#[tokio::main]
async fn main() {
    cli::run_cli(migration::Migrator).await;
}
