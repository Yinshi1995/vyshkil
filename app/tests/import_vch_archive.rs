//! Структурний розбір реального файлу архіву ВЧ (Етап 6) — лише
//! `backend::import::vch_archive::extract` (без БД). Джерело — `source_files/` (ДСК): skip, якщо
//! каталогу нема (00-agent-brief.md). **Лише реальний файл** (рішення користувача, сесія Етапу 6):
//! `docs/source-analysis.md` §1 позначає решту `Дельта/` як синтетично згенеровану.

use std::io::Read;

fn archive_bytes() -> Option<Vec<u8>> {
    let path = std::path::Path::new(
        "../source_files/Дельта/20 АК/ОблікФаховоїПідготовкиАК.xlsx",
    );
    if !path.exists() {
        eprintln!("source_files/ відсутній — тест пропущено (00-agent-brief.md)");
        return None;
    }
    let mut file = std::fs::File::open(path).expect("відкрити файл");
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("прочитати файл");
    Some(bytes)
}

#[test]
fn extracts_real_rows_from_20ak_vch_archive_file() {
    let Some(bytes) = archive_bytes() else { return };

    let rows = app::backend::import::vch_archive::extract(&bytes).expect("розбір архіву ВЧ");

    // Реальний файл (перевірено вручну): 30 рядків, усі "17 овмбр".
    assert!(rows.len() >= 20, "очікував хоч 20 рядків; отримав {}", rows.len());
    for row in &rows {
        assert_eq!(row.org_raw, "17 овмбр", "рядок {}: неочікувана частина", row.row_number);
        assert!(row.start_raw.is_some(), "рядок {}: без дати початку", row.row_number);
        assert!(row.end_raw.is_some(), "рядок {}: без дати завершення", row.row_number);
        let planned: i64 = row.planned_raw.trim().parse().expect("план — число");
        assert!(planned > 0, "рядок {}: план не додатний", row.row_number);
    }
}
