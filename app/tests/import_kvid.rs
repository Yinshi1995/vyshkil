//! Структурний розбір реального файлу "КВід" (03, критерій готовності Етапу 5) — лише
//! `backend::import::kvid::extract` (без БД). Джерело — `source_files/` (ДСК): skip, якщо
//! каталогу нема (00-agent-brief.md).

use std::io::Read;

/// "17 АК 6 додатків.zip": Фах=3, БпС=1, КВід=2 (перевірено вручну — єдиний аркуш "17 АК" з
/// заголовком № з/п/Підрозділ/За штатом/…).
const KVID_ENTRY_INDEX: usize = 2;

fn kvid_bytes() -> Option<Vec<u8>> {
    let zip_path = std::path::Path::new(
        "../source_files/Зразок/Зразки на валідацію/17 АК 6 додатків.zip",
    );
    if !zip_path.exists() {
        eprintln!("source_files/ відсутній — тест пропущено (00-agent-brief.md)");
        return None;
    }
    let file = std::fs::File::open(zip_path).expect("відкрити zip");
    let mut archive = zip::ZipArchive::new(file).expect("прочитати zip");
    let mut entry = archive.by_index(KVID_ENTRY_INDEX).expect("запис КВід у zip");
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).expect("розпакувати запис");
    Some(bytes)
}

#[test]
fn extracts_rows_from_real_kvid_file() {
    let Some(bytes) = kvid_bytes() else { return };

    let rows = app::backend::import::kvid::extract(&bytes).expect("розбір КВід");

    // source-analysis.md не дає точної кількості рядків КВід, але реальний файл (перевірено
    // вручну) має два розділи (безпосереднє/оперативне підпорядкування) по кілька частин кожен.
    assert!(rows.len() >= 8, "очікував хоч кілька частин з обох розділів; отримав {}", rows.len());

    // Жоден секційний ("Військові частини…") чи підсумковий ("ВСЬОГО") рядок не потрапив як дані.
    for row in &rows {
        let org_lower = row.org_raw.to_lowercase();
        assert!(
            !org_lower.contains("військові частини") && !org_lower.contains("всього"),
            "рядок {}:{} схожий на секційний/підсумковий: {:?}",
            row.sheet,
            row.row_number,
            row.org_raw
        );
        assert!(!row.by_tos_raw.is_empty(), "рядок {}:{} без «за штатом»", row.sheet, row.row_number);
        let by_tos: i64 = row.by_tos_raw.trim().parse().expect("«за штатом» — число");
        assert!(by_tos > 0, "рядок {}:{} «за штатом» не додатне", row.sheet, row.row_number);
    }
}
