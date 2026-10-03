//! D6 — звіт передані частини (05 §D6): частини зі статусом `transferred`, кому передані,
//! в яких заходах підготовки беруть участь.

use rust_xlsxwriter::{Formula, Workbook, XlsxError};

use crate::backend::documents::style;
use crate::backend::repo::documents::TransferredOrgRow;

pub fn build_transferred_report(
    workbook: &mut Workbook,
    as_of: &str,
    rows: &[TransferredOrgRow],
) -> Result<(), XlsxError> {
    let ws = workbook.add_worksheet();
    ws.set_name("Передані частини")?;

    ws.set_column_width(0, 5)?;
    ws.set_column_width(1, 25)?;
    ws.set_column_width(2, 25)?;
    ws.set_column_width(3, 12)?;
    ws.set_column_width(4, 12)?;
    ws.set_column_width(5, 12)?;
    ws.set_row_height(0, 28)?;
    ws.set_row_height(2, 22)?;

    let title = format!("Передані частини (станом на {as_of})");
    ws.merge_range(0, 0, 0, 5, &title, &style::title_format())?;

    let hdr = style::header_format();
    for (c, h) in ["№", "Частина", "Кому передана", "БЗВП", "Фахова", "Адаптація"].iter().enumerate() {
        ws.write_string_with_format(2, c as u16, *h, &hdr)?;
    }

    for (i, r) in rows.iter().enumerate() {
        let row = (i + 3) as u32;
        ws.set_row_height(row, 18)?;
        let (txt, num) = if i % 2 == 0 {
            (style::data_format(), style::data_number_format())
        } else {
            (style::data_alt_format(), style::data_alt_number_format())
        };
        ws.write_number_with_format(row, 0, (i + 1) as f64, &num)?;
        ws.write_string_with_format(row, 1, &r.org_label, &txt)?;
        ws.write_string_with_format(row, 2, r.counterpart_label.as_deref().unwrap_or("—"), &txt)?;
        ws.write_number_with_format(row, 3, r.bzvp_count as f64, &num)?;
        ws.write_number_with_format(row, 4, r.special_count as f64, &num)?;
        ws.write_number_with_format(row, 5, r.adaptation_count as f64, &num)?;
    }

    let total_row = (rows.len() + 3) as u32;
    let total_lbl = style::total_format();
    let total_num = style::total_number_format();
    ws.write_string_with_format(total_row, 0, "", &total_lbl)?;
    ws.write_string_with_format(total_row, 1, "ВСЬОГО", &total_lbl)?;
    ws.write_string_with_format(total_row, 2, "", &total_lbl)?;
    for c in 3..=5u16 {
        let col_letter = (b'A' + c as u8) as char;
        ws.write_formula_with_format(
            total_row,
            c,
            Formula::new(format!("SUM({col_letter}4:{col_letter}{total_row})")),
            &total_num,
        )?;
    }

    Ok(())
}
