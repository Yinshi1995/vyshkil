//! Структурний розбір реального файлу "БпС" (03, критерій готовності Етапу 5) — лише
//! `backend::import::bps::extract` (без БД). Джерело — `source_files/` (ДСК): skip, якщо
//! каталогу нема (00-agent-brief.md).

use std::io::Read;

/// "17 АК 6 додатків.zip": Фах=3, БпС=1 (перевірено вручну — аркуші "Завершилась"/"Навчаються").
const BPS_ENTRY_INDEX: usize = 1;

fn bps_bytes() -> Option<Vec<u8>> {
    let zip_path = std::path::Path::new(
        "../source_files/Зразок/Зразки на валідацію/17 АК 6 додатків.zip",
    );
    if !zip_path.exists() {
        eprintln!("source_files/ відсутній — тест пропущено (00-agent-brief.md)");
        return None;
    }
    let file = std::fs::File::open(zip_path).expect("відкрити zip");
    let mut archive = zip::ZipArchive::new(file).expect("прочитати zip");
    let mut entry = archive.by_index(BPS_ENTRY_INDEX).expect("запис БпС у zip");
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).expect("розпакувати запис");
    Some(bytes)
}

#[test]
fn extracts_rows_from_real_bps_file() {
    let Some(bytes) = bps_bytes() else { return };

    let rows = app::backend::import::bps::extract(&bytes).expect("розбір БпС");

    // source-analysis.md §2: "Завершилась"/"Навчаються" — funnel "Викликали → Прибуло до НЦ →
    // Успішно завершило" / "Викликали → Проходять". Не точна кількість (реальний файл міг
    // трохи змінитись) — головне, обидва аркуші дали рядки.
    assert!(!rows.is_empty(), "жодного рядка не розібрано");

    let sheets: std::collections::BTreeSet<_> = rows.iter().map(|r| r.sheet.as_str()).collect();
    assert!(sheets.contains("Завершилась"), "аркуш «Завершилась» не розпізнано");
    assert!(sheets.contains("Навчаються"), "аркуш «Навчаються» не розпізнано");

    // "Всього:" (рядок одразу під заголовком, 03 §3) не потрапив як дані.
    assert!(
        !rows.iter().any(|r| r.org_number_raw.trim().is_empty()),
        "рядок без номера частини потрапив як дані (ймовірно рядок-підсумок)"
    );

    for row in &rows {
        assert!(!row.vos_raw.is_empty(), "рядок {}:{} без ВОС", row.sheet, row.row_number);
        assert!(
            row.start_raw.is_some() && row.end_raw.is_some(),
            "рядок {}:{} без дат",
            row.sheet,
            row.row_number
        );
        let called: i64 = row.called_raw.trim().parse().expect("«Викликали» — число");
        assert!(called > 0, "рядок {}:{} «Викликали» не додатне", row.sheet, row.row_number);
    }

    // "Завершилась" має третю стадію (completed_raw), "Навчаються" — ні.
    assert!(rows.iter().any(|r| r.sheet == "Завершилась" && r.completed_raw.is_some()));
    assert!(rows.iter().filter(|r| r.sheet == "Навчаються").all(|r| r.completed_raw.is_none()));
}
