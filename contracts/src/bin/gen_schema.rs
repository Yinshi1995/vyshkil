//! Генерує JSON Schema для повідомлень, що йдуть через брокер (09-messaging.md §3.7, §4) —
//! джерело істини для TS-типів `services/notifier` (`json-schema-to-typescript` читає ці файли).
//! Ручний запуск: `cargo run -p contracts --features schema-gen --bin gen_schema` (той самий
//! підхід, що `cargo run -p style --bin gen` для CSS — генератор, не частина звичайної збірки).
//! Тест `schema_files_are_up_to_date` (у `tests/schema.rs`) ловить розсинхрон, якщо хтось змінив
//! тип і забув перегенерувати.

use contracts::{
    DiscrepancyOpened, DiscrepancyResolved, Envelope, NotifyResult, NotifySend,
};
use schemars::schema_for;
use std::fs;
use std::path::Path;

// schemars дає всім `Envelope<T>` однаковий `"title": "Envelope"` (не знає конкретної
// інстанціації) -- `json-schema-to-typescript` називає TS-інтерфейс за `title`, тож без цього
// всі 4 файли згенерували б однаково названий тип. Перезаписуємо title тут, а не через
// `#[schemars(title = "...")]` на самому `Envelope<T>` (той один на всі інстанціації однаково).
fn write_schema<T: schemars::JsonSchema>(dir: &Path, filename: &str, title: &str) {
    let mut schema = schema_for!(Envelope<T>);
    schema.insert("title".to_string(), title.into());
    let json = serde_json::to_string_pretty(&schema).expect("серіалізація схеми");
    fs::write(dir.join(filename), json + "\n").expect("запис файлу схеми");
    println!("написано {filename}");
}

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("schema");
    fs::create_dir_all(&dir).expect("створити contracts/schema/");

    write_schema::<NotifySend>(&dir, "notify_send.v1.json", "EnvelopeNotifySendV1");
    write_schema::<NotifyResult>(&dir, "notify_result.v1.json", "EnvelopeNotifyResultV1");
    write_schema::<DiscrepancyOpened>(
        &dir,
        "discrepancy_opened.v1.json",
        "EnvelopeDiscrepancyOpenedV1",
    );
    write_schema::<DiscrepancyResolved>(
        &dir,
        "discrepancy_resolved.v1.json",
        "EnvelopeDiscrepancyResolvedV1",
    );
}
