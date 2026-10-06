//! Переклад технічних серверних помилок імпорту в людські повідомлення (дефект 7: сирий
//! "error running server function: … Zip error: invalid Zip archive: Could not find EOCD" не
//! мав би долітати до людини напряму — технічні деталі лишаються окремо, для адміна, не в
//! основному повідомленні).

/// Розпізнає ВІДОМІ технічні патерни (пошкоджений архів xlsx, неочікувана внутрішня структура) —
/// невпізнане повертає як є (доменні помилки з `backend::import::*` вже написані по-людськи,
/// перекладати їх вдруге не треба).
pub fn humanize_import_error(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("zip") || lower.contains("eocd") {
        "Файл пошкоджений або це не Excel-таблиця (.xlsx)".to_string()
    } else if lower.contains("xml") || lower.contains("calamine") {
        "Файл має незвичну внутрішню структуру — спробуйте зберегти його заново в Excel і \
         завантажити ще раз"
            .to_string()
    } else {
        raw.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip_eocd_error_becomes_human_message() {
        assert_eq!(
            humanize_import_error(
                "error running server function: 1: Zip error: invalid Zip archive: Could not find EOCD"
            ),
            "Файл пошкоджений або це не Excel-таблиця (.xlsx)"
        );
    }

    #[test]
    fn domain_error_passes_through_unchanged() {
        let msg = "у файлі не знайдено жодного рядка даних — перевірте, що це файл \"Фах\"";
        assert_eq!(humanize_import_error(msg), msg);
    }
}
