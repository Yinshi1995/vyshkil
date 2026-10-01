//! D6 — звіт передані частини (05 §D6): частини зі статусом `transferred`, кому передані,
//! в яких заходах підготовки беруть участь.

use rust_xlsxwriter::{Format, FormatAlign, Formula, Workbook, XlsxError};

use crate::backend::repo::documents::TransferredOrgRow;

pub fn build_transferred_report(
    workbook: &mut Workbook,
    as_of: &str,
    rows: &[TransferredOrgRow],
) -> Result<(), XlsxError> {
    let ws = workbook.add_worksheet();
    ws.set_name("Передані частини")?;

    let hdr = Format::new().set_bold().set_align(FormatAlign::Center);
    let bold = Format::new().set_bold();

    let title = format!("Передані частини (станом на {as_of})");
    ws.merge_range(0, 0, 0, 5, &title, &bold)?;

    for (c, h) in ["№", "Частина", "Кому передана", "БЗВП", "Фахова", "Адаптація"].iter().enumerate() {
        ws.write_string_with_format(2, c as u16, *h, &hdr)?;
    }

    for (i, r) in rows.iter().enumerate() {
        let row = (i + 3) as u32;
        ws.write_number(row, 0, (i + 1) as f64)?;
        ws.write_string(row, 1, &r.org_label)?;
        ws.write_string(row, 2, r.counterpart_label.as_deref().unwrap_or("—"))?;
        ws.write_number(row, 3, r.bzvp_count as f64)?;
        ws.write_number(row, 4, r.special_count as f64)?;
        ws.write_number(row, 5, r.adaptation_count as f64)?;
    }

    let total_row = (rows.len() + 3) as u32;
    ws.write_string_with_format(total_row, 1, "ВСЬОГО", &bold)?;
    for c in 3..=5u16 {
        let col_letter = (b'A' + c as u8) as char;
        ws.write_formula(
            total_row,
            c,
            Formula::new(format!("SUM({col_letter}4:{col_letter}{total_row})")),
        )?;
    }

    Ok(())
}
