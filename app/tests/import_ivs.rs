//! Структурний розбір реального файлу "ІВС" (03, критерій готовності Етапу 5) — лише
//! `backend::import::ivs::extract` (без БД). Джерело — `source_files/` (ДСК): skip, якщо
//! каталогу нема (00-agent-brief.md).

use std::io::Read;

/// "17 АК 6 додатків.zip": ІВС=0, БпС=1, КВід=2, Фах=3 (перевірено вручну — єдиний аркуш "Аркуш1"
/// з ДВОМА таблицями: укомплектованість + курси).
const IVS_ENTRY_INDEX: usize = 0;

fn ivs_bytes() -> Option<Vec<u8>> {
    let zip_path = std::path::Path::new(
        "../source_files/Зразок/Зразки на валідацію/17 АК 6 додатків.zip",
    );
    if !zip_path.exists() {
        eprintln!("source_files/ відсутній — тест пропущено (00-agent-brief.md)");
        return None;
    }
    let file = std::fs::File::open(zip_path).expect("відкрити zip");
    let mut archive = zip::ZipArchive::new(file).expect("прочитати zip");
    let mut entry = archive.by_index(IVS_ENTRY_INDEX).expect("запис ІВС у zip");
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).expect("розпакувати запис");
    Some(bytes)
}

#[test]
fn extracts_all_three_shapes_from_real_ivs_file() {
    let Some(bytes) = ivs_bytes() else { return };

    let extract = app::backend::import::ivs::extract(&bytes).expect("розбір ІВС");

    // Реальний файл (перевірено вручну): дві секції підпорядкування, 13 частин разом.
    assert!(
        extract.staffing.len() >= 10,
        "очікував хоч десяток частин з обох секцій; отримав {}",
        extract.staffing.len()
    );
    for row in &extract.staffing {
        let org_lower = row.org_raw.to_lowercase();
        assert!(
            !org_lower.contains("військові частини") && !org_lower.contains("всього"),
            "рядок схожий на секційний/підсумковий: {:?}",
            row.org_raw
        );
        let by_tos: i64 = row.by_tos_raw.trim().parse().expect("«за штатом» — число");
        assert!(by_tos > 0, "«{}»: «за штатом» не додатне", row.org_raw);
    }

    // Стажування є лише в кількох рядках прямого підпорядкування (реальний файл, перевірено
    // вручну) — не в кожному, але хоч кілька мають бути розпізнані.
    assert!(!extract.internships.is_empty(), "очікував хоч одне стажування, розпізнане з тексту");
    for i in &extract.internships {
        assert!(i.count_raw.parse::<i64>().is_ok_and(|n| n > 0), "стажування без додатної кількості");
        assert!(!i.start_raw.is_empty() && !i.end_raw.is_empty(), "стажування без обох дат");
    }

    // Курси (друга таблиця, "Курс"/"Кількість"/"Період навчання") — реальний файл: 7 курсів.
    assert!(extract.courses.len() >= 5, "очікував хоч кілька курсів; отримав {}", extract.courses.len());
    for c in &extract.courses {
        assert!(!c.course_raw.is_empty(), "курс без назви");
        assert!(c.count_raw.parse::<i64>().is_ok_and(|n| n > 0), "«{}»: кількість не додатна", c.course_raw);
        assert!(c.start_raw.is_some() && c.end_raw.is_some(), "«{}»: курс без обох дат", c.course_raw);
    }
}
