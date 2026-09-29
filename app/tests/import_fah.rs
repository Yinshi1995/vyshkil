//! Структурний розбір реального файлу "Фах" (03, критерій готовності Етапу 5) — лише
//! `backend::import::fah::extract` (без БД, без резолюції org/vos/site). Джерело — `source_files/`
//! (ДСК): skip, якщо каталогу нема (00-agent-brief.md).

use std::io::Read;

/// "17 АК 6 додатків.zip" містить 4 файли (Фах/БпС/КВід/ІВС); імена записів у zip — CP866, не
/// UTF-8/CP437, тож читаємо за ПОЗИЦІЄЮ запису, не за іменем (перевірено вручну: аркуші
/// "Пройшли"/"Проходять" -- ознака Фах -- у записі з індексом 3).
const FAH_ENTRY_INDEX: usize = 3;

fn fah_bytes() -> Option<Vec<u8>> {
    let zip_path = std::path::Path::new(
        "../source_files/Зразок/Зразки на валідацію/17 АК 6 додатків.zip",
    );
    if !zip_path.exists() {
        eprintln!("source_files/ відсутній — тест пропущено (00-agent-brief.md)");
        return None;
    }
    let file = std::fs::File::open(zip_path).expect("відкрити zip");
    let mut archive = zip::ZipArchive::new(file).expect("прочитати zip");
    let mut entry = archive.by_index(FAH_ENTRY_INDEX).expect("запис Фах у zip");
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).expect("розпакувати запис");
    Some(bytes)
}

#[test]
fn extracts_all_rows_from_real_fah_file() {
    let Some(bytes) = fah_bytes() else { return };

    let rows = app::backend::import::fah::extract(&bytes).expect("розбір Фах");

    // 03 §3: рядки "<частина> Підсумок", підписи, службовий гриф — не дані. Джерело описує
    // 758+148 "живих" рядків (docs/source-analysis.md §2) — точну кількість беремо як нижню межу
    // (реальний файл міг трохи змінитись), головне — розбір взагалі щось знайшов на ОБОХ аркушах.
    assert!(rows.len() > 800, "очікував і Пройшли (758), і Проходять (148); отримав {}", rows.len());

    let sheets: std::collections::BTreeSet<_> = rows.iter().map(|r| r.sheet.as_str()).collect();
    assert!(sheets.contains("Пройшли"));
    assert!(sheets.contains("Проходять"));

    // Кожен рядок — реальні дані, не сміття: організація присутня, кількість додатна. Дати
    // здебільшого теж є (Фах зберігає їх як справжні Excel-дати), але кілька клітинок у живому
    // файлі набрані текстом ("2026-08-03" замість дати) чи порожні — це РЕАЛЬНИЙ дефект джерела
    // (той самий клас, що 03 §5 явно згадує), не помилка розбору: рахуємо, не вимагаємо нуля.
    let mut missing_dates = 0;
    for row in &rows {
        assert!(!row.org_raw.is_empty(), "рядок {}:{} без частини", row.sheet, row.row_number);
        if row.start_raw.is_none() || row.end_raw.is_none() {
            missing_dates += 1;
        }
        let count: i64 = row.count_raw.parse().expect("кількість — число");
        assert!(count > 0, "рядок {}:{} кількість не додатна", row.sheet, row.row_number);
    }
    assert!(
        missing_dates < rows.len() / 10,
        "забагато рядків без дат ({missing_dates} з {}) — схоже на системну помилку розбору, не поодинокий дефект джерела",
        rows.len()
    );

    // Жодного "Підсумок"/підписного рядка не потрапило як дані (03 §3).
    assert!(
        !rows.iter().any(|r| r.org_raw.contains("Підсумок")),
        "рядок-підсумок потрапив як дані"
    );
}
