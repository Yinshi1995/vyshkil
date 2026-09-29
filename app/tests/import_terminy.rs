//! Структурний розбір реального файлу "Терміни" (03, критерій готовності Етапу 5 — останній із
//! п'яти файлів) — лише `backend::import::terminy::extract` (без БД). Джерело — `source_files/`
//! (ДСК), окремий файл (НЕ в zip-архіві з рештою чотирьох): skip, якщо його нема (00-agent-brief.md).

use std::io::Read;

fn terminy_bytes() -> Option<Vec<u8>> {
    let path = std::path::Path::new(
        "../source_files/Зразок/Зразки на валідацію/\
         17 АК Терміни проведення підготовки 25.09.2026 (для УВ(с)).xlsx",
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
fn extracts_all_three_parallel_lists_from_real_terminy_file() {
    let Some(bytes) = terminy_bytes() else { return };

    let extract = app::backend::import::terminy::extract(&bytes).expect("розбір Терміни");

    // Реальний файл (перевірено вручну): десятки підрозділів, кожен з різною кількістю рядків
    // у кожному з трьох списків — Фахова найбільша (по ВОС), БЗВП і Адаптація менші.
    assert!(extract.bzvp.len() >= 10, "БЗВП: очікував хоч 10 рядків, отримав {}", extract.bzvp.len());
    assert!(
        extract.special.len() >= 20,
        "Фахова: очікував хоч 20 рядків, отримав {}",
        extract.special.len()
    );
    assert!(!extract.adapt.is_empty(), "Адаптація: очікував хоч один рядок");

    for row in &extract.bzvp {
        assert!(!row.org_raw.is_empty(), "БЗВП рядок без підрозділу");
        assert!(!row.org_raw.to_lowercase().contains("всього"), "«Всього» потрапило як БЗВП рядок");
        let n: i64 = row.count_raw.parse().expect("кількість — число");
        assert!(n > 0, "«{}»: кількість не додатна", row.org_raw);
        assert!(!row.term_raw.is_empty(), "«{}»: БЗВП без термінів", row.org_raw);
    }
    for row in &extract.special {
        assert!(!row.org_raw.is_empty(), "Фахова рядок без підрозділу");
        assert!(!row.org_raw.to_lowercase().contains("всього"), "«Всього» потрапило як Фахова рядок");
        let n: i64 = row.count_raw.parse().expect("кількість — число");
        assert!(n > 0, "«{}»: кількість не додатна", row.org_raw);
        // Не кожен рядок має "ВОС" у колонці спеціальності — реальний дефект джерела
        // (docs/source-analysis.md: назви курсів трапляються в колонці ВОС), extract_vos_code
        // тоді дає None і рядок лишається text-only для ручного підтвердження (03 §5) — це
        // очікувано, не помилка екстрактора.
        assert!(!row.specialty_raw.is_empty(), "«{}»: спеціальність порожня", row.org_raw);
    }
    for row in &extract.adapt {
        assert!(!row.org_raw.is_empty(), "Адаптація рядок без підрозділу");
        assert!(!row.org_raw.to_lowercase().contains("всього"), "«Всього» потрапило як Адаптація рядок");
        let n: i64 = row.count_raw.parse().expect("кількість — число");
        assert!(n > 0, "«{}»: кількість не додатна", row.org_raw);
    }
}
