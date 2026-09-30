use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Транзакційний outbox (09-messaging.md §3.1, Фаза 1 брокера): доменна дія пише свої дані Й рядок
// outbox В ОДНІЙ транзакції -- relay (фонова задача в `server`) публікує в NATS JetStream окремо,
// поза цією транзакцією. `id` -- Uuid v7, ГЕНЕРУЄТЬСЯ Rust-кодом (не DB DEFAULT), той самий
// рядок стає значенням заголовка `Nats-Msg-Id` при публікації (дедуплікація на боці брокера,
// якщо relay впаде між "опубліковано" й "позначено published_at").
//
// БЕЗ audit-тригера -- та сама причина, що `reported_group`/`generated_document`: технічна
// таблиця релею, яку змінює фонова задача (не людина-актор через SET LOCAL app.actor), рядки не
// редагуються користувачем.
//
// БЕЗ `LISTEN/NOTIFY`-тригера (§3.1 згадує його як опцію поруч із таймером): relay читає raw
// `LISTEN`-з'єднання окремо від sea-orm-пулу (sea-orm сам його не підтримує) -- зайва складність
// заради латентності, яка тут не критична (внутрішній інструмент обліку, не HFT). Relay --
// простий таймер-опитувач (`server/src/relay.rs`), рішення задокументоване там і в
// `.claude/decisions/broker-nats-jetstream.md`.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Outbox::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Outbox::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Outbox::Subject).text().not_null())
                    .col(ColumnDef::new(Outbox::Payload).json_binary().not_null())
                    .col(ColumnDef::new(Outbox::Headers).json_binary().not_null())
                    .col(
                        ColumnDef::new(Outbox::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Outbox::PublishedAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(Outbox::Attempts).integer().not_null().default(0))
                    .col(ColumnDef::new(Outbox::LastError).text().null())
                    .to_owned(),
            )
            .await?;

        // Партціальний індекс -- relay читає ЛИШЕ непубліковані, і саме в порядку створення
        // (`ORDER BY id` -- Uuid v7 сортується як час, той самий трюк, що вже дає нам "insert
        // order" без окремого serial-стовпця).
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-outbox-unpublished")
                    .table(Outbox::Table)
                    .col(Outbox::Id)
                    .and_where(Expr::col(Outbox::PublishedAt).is_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Outbox::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum Outbox {
    Table,
    Id,
    Subject,
    Payload,
    Headers,
    CreatedAt,
    PublishedAt,
    Attempts,
    LastError,
}
