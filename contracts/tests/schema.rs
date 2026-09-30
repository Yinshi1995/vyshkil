//! "Чи не застарів `contracts/schema/*.json`" (09-messaging.md §6) — регенерує схему в пам'яті й
//! звіряє із закомiченим файлом; розсинхрон = хтось змінив тип у `contracts` і забув перегенерувати.
//! Запуск: `cargo test -p contracts --features schema-gen`.
#![cfg(feature = "schema-gen")]

use contracts::{DiscrepancyOpened, DiscrepancyResolved, Envelope, NotifyResult, NotifySend};
use schemars::schema_for;
use std::fs;
use std::path::Path;

// `title` тут МУСИТЬ збігатись із тим, що встановлює `src/bin/gen_schema.rs::write_schema` —
// дублювання рядка title, не логіки генерації; розсинхрон цих двох рядків так само провалить
// тест (обидва мають say the same thing), як і розсинхрон полів типу.
fn assert_schema_up_to_date<T: schemars::JsonSchema>(filename: &str, title: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("schema").join(filename);
    let committed = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("{filename} не знайдено — запусти gen_schema (див. doc-comment у src/bin/gen_schema.rs)"));
    let mut schema = schema_for!(Envelope<T>);
    schema.insert("title".to_string(), title.into());
    let fresh = serde_json::to_string_pretty(&schema).expect("серіалізація схеми") + "\n";
    assert_eq!(
        committed, fresh,
        "{filename} застарів — запусти: cargo run -p contracts --features schema-gen --bin gen_schema"
    );
}

#[test]
fn schema_files_are_up_to_date() {
    assert_schema_up_to_date::<NotifySend>("notify_send.v1.json", "EnvelopeNotifySendV1");
    assert_schema_up_to_date::<NotifyResult>("notify_result.v1.json", "EnvelopeNotifyResultV1");
    assert_schema_up_to_date::<DiscrepancyOpened>(
        "discrepancy_opened.v1.json",
        "EnvelopeDiscrepancyOpenedV1",
    );
    assert_schema_up_to_date::<DiscrepancyResolved>(
        "discrepancy_resolved.v1.json",
        "EnvelopeDiscrepancyResolvedV1",
    );
}
