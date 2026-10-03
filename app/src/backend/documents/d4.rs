//! D4 "Підготовка" (05 §D4) — презентація pptx: титульний + загальна динаміка + по одному
//! слайду з таблицею на кожний орган. Стиль: кремовий фон (#F5F2EC), оранжевий акцент (#F39200),
//! темна олива (#453F2E) для заголовків таблиць, пастельне чергування рядків (#F5F2EC / #FFFFFF) —
//! за зразками замовника (reference_document_style). Шрифт Arial.
//!
//! Людей поіменно тут немає — лише кількості груп (CLAUDE.md, жорсткі правила).

use std::io::{Cursor, Write};

use chrono::NaiveDate;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::backend::repo::documents::{DailyRollup, KindCounts, OrgRollupRow};
use crate::domain::format::{signed_delta, thousands};

pub struct CorpsSlide {
    pub label: String,
    pub today: DailyRollup,
    pub yesterday: DailyRollup,
}

// ---------------------------------------------------------------------------
// Dimensions (16:9, 13.3" × 7.5" — matches sample presentations)
// ---------------------------------------------------------------------------

const EMU_CM: i64 = 360000;
const SLIDE_W: i64 = 33_87 * EMU_CM / 100;
const SLIDE_H: i64 = 19_05 * EMU_CM / 100;

// ---------------------------------------------------------------------------
// Palette — cream/pastel military style from sample presentations
// ---------------------------------------------------------------------------

const BG_HEADER: &str = "453F2E";
const BG_DATA_EVEN: &str = "F5F2EC";
const BG_DATA_ODD: &str = "FFFFFF";
const BG_TOTAL: &str = "F39200";
const BG_TILE: &str = "FFFFFF";
const TEXT_DARK: &str = "302D24";
const TEXT_WHITE: &str = "FFFFFF";
const TEXT_MUTED: &str = "9C947F";
const TEXT_ORANGE: &str = "F39200";
const TEXT_AMBER: &str = "855000";
const BORDER: &str = "C7C1B0";

// ---------------------------------------------------------------------------
// Aggregation
// ---------------------------------------------------------------------------

fn sum_kind(rollup: &DailyRollup) -> (KindCounts, KindCounts, KindCounts) {
    let mut bzvp = KindCounts::default();
    let mut special = KindCounts::default();
    let mut adaptation = KindCounts::default();
    for row in rollup.main.iter().chain(rollup.out_of_zone.iter()) {
        bzvp.total += row.bzvp.total;
        bzvp.finishing_today += row.bzvp.finishing_today;
        bzvp.started_today += row.bzvp.started_today;
        special.total += row.special.total;
        special.finishing_today += row.special.finishing_today;
        special.started_today += row.special.started_today;
        adaptation.total += row.adaptation.total;
        adaptation.finishing_today += row.adaptation.finishing_today;
        adaptation.started_today += row.adaptation.started_today;
    }
    (bzvp, special, adaptation)
}

fn format_changes(k: &KindCounts) -> String {
    let f = k.finishing_today;
    let s = k.started_today;
    if f == 0 && s == 0 {
        return "\u{2014}".to_string();
    }
    let mut out = String::new();
    if f > 0 {
        out.push_str(&f.to_string());
    }
    if s > 0 {
        if !out.is_empty() {
            out.push('/');
        }
        out.push('+');
        out.push_str(&s.to_string());
    }
    if out.is_empty() {
        "\u{2014}".to_string()
    } else {
        out
    }
}

// ---------------------------------------------------------------------------
// Low-level XML helpers
// ---------------------------------------------------------------------------

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn text_run(text: &str, font_size: u32, color: &str, bold: bool) -> String {
    let b = if bold { " b=\"1\"" } else { "" };
    format!(
        "<a:r><a:rPr lang=\"uk-UA\" sz=\"{font_size}\" dirty=\"0\"{b}>\
         <a:solidFill><a:srgbClr val=\"{color}\"/></a:solidFill>\
         <a:latin typeface=\"Arial\"/><a:cs typeface=\"Arial\"/></a:rPr>\
         <a:t>{}</a:t></a:r>",
        xml_escape(text)
    )
}

fn text_box(x: i64, y: i64, w: i64, h: i64, name: &str, runs: &str) -> String {
    format!(
        "<p:sp><p:nvSpPr><p:cNvPr id=\"0\" name=\"{name}\"/>\
         <p:cNvSpPr txBox=\"1\"/><p:nvPr/></p:nvSpPr>\
         <p:spPr><a:xfrm><a:off x=\"{x}\" y=\"{y}\"/>\
         <a:ext cx=\"{w}\" cy=\"{h}\"/></a:xfrm>\
         <a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom>\
         <a:noFill/></p:spPr>\
         <p:txBody><a:bodyPr wrap=\"square\" rtlCol=\"0\"/>\
         <a:lstStyle/><a:p>{runs}</a:p></p:txBody></p:sp>"
    )
}

fn text_box_centered(x: i64, y: i64, w: i64, h: i64, name: &str, runs: &str) -> String {
    format!(
        "<p:sp><p:nvSpPr><p:cNvPr id=\"0\" name=\"{name}\"/>\
         <p:cNvSpPr txBox=\"1\"/><p:nvPr/></p:nvSpPr>\
         <p:spPr><a:xfrm><a:off x=\"{x}\" y=\"{y}\"/>\
         <a:ext cx=\"{w}\" cy=\"{h}\"/></a:xfrm>\
         <a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom>\
         <a:noFill/></p:spPr>\
         <p:txBody><a:bodyPr wrap=\"square\" rtlCol=\"0\" anchor=\"ctr\"/>\
         <a:lstStyle/><a:p><a:pPr algn=\"ctr\"/>{runs}</a:p></p:txBody></p:sp>"
    )
}

fn rect_fill(x: i64, y: i64, w: i64, h: i64, color: &str) -> String {
    format!(
        "<p:sp><p:nvSpPr><p:cNvPr id=\"0\" name=\"r\"/>\
         <p:cNvSpPr/><p:nvPr/></p:nvSpPr>\
         <p:spPr><a:xfrm><a:off x=\"{x}\" y=\"{y}\"/>\
         <a:ext cx=\"{w}\" cy=\"{h}\"/></a:xfrm>\
         <a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom>\
         <a:solidFill><a:srgbClr val=\"{color}\"/></a:solidFill>\
         <a:ln><a:noFill/></a:ln></p:spPr>\
         <p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr/></a:p></p:txBody></p:sp>"
    )
}

fn connector_line(x: i64, y: i64, w: i64, h: i64, color: &str, weight: i64) -> String {
    format!(
        "<p:cxnSp><p:nvCxnSpPr><p:cNvPr id=\"0\" name=\"cn\"/>\
         <p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr>\
         <p:spPr><a:xfrm><a:off x=\"{x}\" y=\"{y}\"/>\
         <a:ext cx=\"{w}\" cy=\"{h}\"/></a:xfrm>\
         <a:prstGeom prst=\"line\"><a:avLst/></a:prstGeom>\
         <a:ln w=\"{weight}\"><a:solidFill><a:srgbClr val=\"{color}\"/></a:solidFill></a:ln>\
         </p:spPr></p:cxnSp>"
    )
}

// ---------------------------------------------------------------------------
// Table XML helpers
// ---------------------------------------------------------------------------

fn table_cell(text: &str, font_sz: u32, color: &str, bold: bool, bg: &str, align: &str) -> String {
    let b = if bold { " b=\"1\"" } else { "" };
    format!(
        "<a:tc><a:txBody><a:bodyPr anchor=\"ctr\"/><a:lstStyle/>\
         <a:p><a:pPr algn=\"{align}\"/>\
         <a:r><a:rPr lang=\"uk-UA\" sz=\"{font_sz}\" dirty=\"0\"{b}>\
         <a:solidFill><a:srgbClr val=\"{color}\"/></a:solidFill>\
         <a:latin typeface=\"Arial\"/><a:cs typeface=\"Arial\"/></a:rPr>\
         <a:t>{}</a:t></a:r></a:p></a:txBody>\
         <a:tcPr marL=\"45720\" marR=\"45720\" marT=\"18288\" marB=\"18288\">\
         <a:solidFill><a:srgbClr val=\"{bg}\"/></a:solidFill>\
         <a:lnL w=\"6350\" cmpd=\"sng\"><a:solidFill><a:srgbClr val=\"{BORDER}\"/></a:solidFill></a:lnL>\
         <a:lnR w=\"6350\" cmpd=\"sng\"><a:solidFill><a:srgbClr val=\"{BORDER}\"/></a:solidFill></a:lnR>\
         <a:lnT w=\"6350\" cmpd=\"sng\"><a:solidFill><a:srgbClr val=\"{BORDER}\"/></a:solidFill></a:lnT>\
         <a:lnB w=\"6350\" cmpd=\"sng\"><a:solidFill><a:srgbClr val=\"{BORDER}\"/></a:solidFill></a:lnB>\
         </a:tcPr></a:tc>",
        xml_escape(text)
    )
}

fn table_row(cells: &str, height: i64) -> String {
    format!("<a:tr h=\"{height}\">{cells}</a:tr>")
}

fn table_frame(x: i64, y: i64, w: i64, h: i64, col_widths: &[i64], rows: &str) -> String {
    let grid: String = col_widths.iter().map(|cw| format!("<a:gridCol w=\"{cw}\"/>")).collect();
    format!(
        "<p:graphicFrame>\
         <p:nvGraphicFramePr><p:cNvPr id=\"0\" name=\"Table\"/>\
         <p:cNvGraphicFramePr><a:graphicFrameLocks noGrp=\"1\"/></p:cNvGraphicFramePr>\
         <p:nvPr/></p:nvGraphicFramePr>\
         <p:xfrm><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"{w}\" cy=\"{h}\"/></p:xfrm>\
         <a:graphic>\
         <a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/table\">\
         <a:tbl><a:tblPr firstRow=\"1\" bandRow=\"1\"><a:noFill/></a:tblPr>\
         <a:tblGrid>{grid}</a:tblGrid>\
         {rows}\
         </a:tbl></a:graphicData></a:graphic></p:graphicFrame>"
    )
}

// ---------------------------------------------------------------------------
// Slide furniture
// ---------------------------------------------------------------------------

fn slide_bg() -> &'static str {
    "<p:bg><p:bgPr><a:solidFill><a:srgbClr val=\"F5F2EC\"/></a:solidFill>\
     <a:effectLst/></p:bgPr></p:bg>"
}

fn wrap_slide(shapes: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<p:sld xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\">\
<p:cSld>{}<p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/>\
         <p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>\
         <p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/>\
         <a:ext cx=\"0\" cy=\"0\"/><a:chOff x=\"0\" y=\"0\"/>\
         <a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr>\
         {}</p:spTree></p:cSld></p:sld>",
        slide_bg(),
        shapes
    )
}

fn dsk_label(y: i64) -> String {
    text_box(
        SLIDE_W - 500 * EMU_CM / 100,
        y,
        460 * EMU_CM / 100,
        80 * EMU_CM / 100,
        "dsk",
        &text_run("ДЛЯ СЛУЖБОВОГО КОРИСТУВАННЯ", 800, TEXT_MUTED, false),
    )
}

fn page_counter(num: usize, total: usize) -> String {
    let x = SLIDE_W - 250 * EMU_CM / 100;
    let y = SLIDE_H - 120 * EMU_CM / 100;
    text_box(x, y, 220 * EMU_CM / 100, 80 * EMU_CM / 100, "pc",
        &text_run(&format!("{num} / {total}"), 850, TEXT_MUTED, false))
}

// ---------------------------------------------------------------------------
// KPI tile (white card with orange accent bar)
// ---------------------------------------------------------------------------

fn kpi_tile(x: i64, y: i64, w: i64, label: &str, value: &str, sub: &str) -> String {
    let h = 180 * EMU_CM / 100;
    let accent = rect_fill(x, y, w, 18000, TEXT_ORANGE);
    let bg = rect_fill(x, y + 18000, w, h - 18000, BG_TILE);
    let border = format!(
        "<p:sp><p:nvSpPr><p:cNvPr id=\"0\" name=\"tb\"/>\
         <p:cNvSpPr/><p:nvPr/></p:nvSpPr>\
         <p:spPr><a:xfrm><a:off x=\"{x}\" y=\"{y}\"/>\
         <a:ext cx=\"{w}\" cy=\"{h}\"/></a:xfrm>\
         <a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom>\
         <a:noFill/>\
         <a:ln w=\"6350\"><a:solidFill><a:srgbClr val=\"{BORDER}\"/></a:solidFill></a:ln>\
         </p:spPr>\
         <p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr/></a:p></p:txBody></p:sp>"
    );
    let val = text_box(
        x + 15 * EMU_CM / 100, y + 15 * EMU_CM / 100,
        w - 30 * EMU_CM / 100, 70 * EMU_CM / 100,
        "kv", &text_run(value, 2800, TEXT_ORANGE, true));
    let lbl = text_box(
        x + 15 * EMU_CM / 100, y + 80 * EMU_CM / 100,
        w - 30 * EMU_CM / 100, 40 * EMU_CM / 100,
        "kl", &text_run(label, 1100, TEXT_DARK, true));
    let detail = text_box(
        x + 15 * EMU_CM / 100, y + 120 * EMU_CM / 100,
        w - 30 * EMU_CM / 100, 50 * EMU_CM / 100,
        "kd", &text_run(sub, 900, TEXT_MUTED, false));
    format!("{bg}{accent}{border}{val}{lbl}{detail}")
}

// ---------------------------------------------------------------------------
// Title slide
// ---------------------------------------------------------------------------

fn title_slide(date: NaiveDate) -> String {
    let dsk = dsk_label(30 * EMU_CM / 100);

    let accent_line = rect_fill(
        400 * EMU_CM / 100,
        480 * EMU_CM / 100,
        SLIDE_W - 800 * EMU_CM / 100,
        18000,
        TEXT_ORANGE,
    );

    let org = text_box_centered(
        200 * EMU_CM / 100, 520 * EMU_CM / 100,
        SLIDE_W - 400 * EMU_CM / 100, 300 * EMU_CM / 100,
        "org",
        &text_run("УГРУПОВАННЯ ВІЙСЬК (СИЛ) \"ПІВДЕНЬ\"", 2600, TEXT_DARK, true),
    );

    let title = text_box_centered(
        200 * EMU_CM / 100, 850 * EMU_CM / 100,
        SLIDE_W - 400 * EMU_CM / 100, 200 * EMU_CM / 100,
        "title",
        &text_run("ПІДГОТОВКА", 3400, TEXT_ORANGE, true),
    );

    let tags = text_box_centered(
        200 * EMU_CM / 100, 1100 * EMU_CM / 100,
        SLIDE_W - 400 * EMU_CM / 100, 100 * EMU_CM / 100,
        "tags",
        &text_run("БЗВП  \u{00B7}  ФАХОВА ПІДГОТОВКА  \u{00B7}  АДАПТАЦІЯ", 1200, BORDER, false),
    );

    let dt = text_box_centered(
        200 * EMU_CM / 100, 1300 * EMU_CM / 100,
        SLIDE_W - 400 * EMU_CM / 100, 100 * EMU_CM / 100,
        "date",
        &text_run(
            &date.format("СТАНОМ НА %d.%m.%Y").to_string().to_uppercase(),
            1400, TEXT_ORANGE, true,
        ),
    );

    wrap_slide(&format!("{dsk}{accent_line}{org}{title}{tags}{dt}"))
}

// ---------------------------------------------------------------------------
// Overview slide (KPI tiles)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn overview_slide(
    date: NaiveDate,
    total_slides: usize,
    bzvp_t: i64, bzvp_y: i64,
    special_t: i64, special_y: i64,
    adapt_t: i64, adapt_y: i64,
) -> String {
    let dsk = dsk_label(30 * EMU_CM / 100);
    let pg = page_counter(2, total_slides);

    let header = text_box(
        120 * EMU_CM / 100, 50 * EMU_CM / 100,
        SLIDE_W - 700 * EMU_CM / 100, 100 * EMU_CM / 100,
        "hdr",
        &text_run("ЗАГАЛЬНА ДИНАМІКА ПІДГОТОВКИ", 1800, TEXT_ORANGE, true),
    );
    let sub_hdr = text_box(
        120 * EMU_CM / 100, 140 * EMU_CM / 100,
        SLIDE_W - 700 * EMU_CM / 100, 60 * EMU_CM / 100,
        "sub",
        &text_run(
            &format!("Станом на {}", date.format("%d.%m.%Y")),
            1100, TEXT_MUTED, false,
        ),
    );

    let accent = connector_line(
        120 * EMU_CM / 100, 210 * EMU_CM / 100,
        SLIDE_W - 240 * EMU_CM / 100, 0, TEXT_ORANGE, 19050,
    );

    let left = 120 * EMU_CM / 100;
    let tile_w = 360 * EMU_CM / 100;
    let gap = 380 * EMU_CM / 100;
    let top = 280 * EMU_CM / 100;

    let total_t = bzvp_t + special_t + adapt_t;
    let total_y = bzvp_y + special_y + adapt_y;

    let tiles = [
        kpi_tile(left, top, tile_w, "УСЬОГО ЗАЛУЧЕНО",
            &thousands(total_t),
            &format!("зміна за добу: {}", signed_delta(total_t - total_y))),
        kpi_tile(left + gap, top, tile_w, "БЗВП",
            &thousands(bzvp_t),
            &format!("{} \u{2192} {}", thousands(bzvp_y), thousands(bzvp_t))),
        kpi_tile(left + gap * 2, top, tile_w, "ФАХОВА ПІДГОТОВКА",
            &thousands(special_t),
            &format!("{} \u{2192} {}", thousands(special_y), thousands(special_t))),
        kpi_tile(left + gap * 3, top, tile_w, "АДАПТАЦІЯ",
            &thousands(adapt_t),
            &format!("{} \u{2192} {}", thousands(adapt_y), thousands(adapt_t))),
    ];

    let dsk_bottom = text_box(
        120 * EMU_CM / 100, SLIDE_H - 120 * EMU_CM / 100,
        500 * EMU_CM / 100, 80 * EMU_CM / 100,
        "dskb",
        &text_run("ДЛЯ СЛУЖБОВОГО КОРИСТУВАННЯ", 800, TEXT_MUTED, false),
    );

    let mut s = format!("{dsk}{pg}{header}{sub_hdr}{accent}");
    for tile in &tiles {
        s.push_str(tile);
    }
    s.push_str(&dsk_bottom);
    wrap_slide(&s)
}

// ---------------------------------------------------------------------------
// Corps slide — real PowerPoint table
// ---------------------------------------------------------------------------

const COL_NAME_W: i64 = 3100000;
const COL_NUM_W: i64 = 1500000;
const COL_CHG_W: i64 = 1100000;
const TABLE_Y: i64 = 850 * EMU_CM / 100;
const ROW_HEADER_H: i64 = 300000;
const ROW_DATA_H: i64 = 220000;
const ROW_TOTAL_H: i64 = 260000;

fn table_w() -> i64 {
    COL_NAME_W + (COL_NUM_W + COL_CHG_W) * 3
}

fn col_widths() -> Vec<i64> {
    vec![COL_NAME_W, COL_NUM_W, COL_CHG_W, COL_NUM_W, COL_CHG_W, COL_NUM_W, COL_CHG_W]
}

fn header_row() -> String {
    let c = |text: &str| table_cell(text, 900, TEXT_WHITE, true, BG_HEADER, "ctr");
    let cells = [
        table_cell("Підрозділ", 900, TEXT_WHITE, true, BG_HEADER, "l"),
        c("БЗВП"), c("Зміни"),
        c("Фахова"), c("Зміни"),
        c("Адаптація"), c("Зміни"),
    ];
    table_row(&cells.join(""), ROW_HEADER_H)
}

fn data_row(row: &OrgRollupRow, even: bool) -> String {
    let bg = if even { BG_DATA_EVEN } else { BG_DATA_ODD };
    let name = table_cell(&row.org_label, 850, TEXT_DARK, false, bg, "l");
    let n = |k: &KindCounts| table_cell(&thousands(k.total), 850, TEXT_DARK, false, bg, "ctr");
    let ch = |k: &KindCounts| {
        let txt = format_changes(k);
        let color = if k.started_today > 0 { TEXT_AMBER } else { TEXT_DARK };
        table_cell(&txt, 850, color, false, bg, "ctr")
    };
    let cells = [
        name,
        n(&row.bzvp), ch(&row.bzvp),
        n(&row.special), ch(&row.special),
        n(&row.adaptation), ch(&row.adaptation),
    ];
    table_row(&cells.join(""), ROW_DATA_H)
}

fn total_row(bt: &KindCounts, st: &KindCounts, at: &KindCounts) -> String {
    let c = |text: &str| table_cell(text, 900, TEXT_WHITE, true, BG_TOTAL, "ctr");
    let cells = [
        table_cell("ВСЬОГО", 900, TEXT_WHITE, true, BG_TOTAL, "l"),
        c(&thousands(bt.total)), c(&format_changes(bt)),
        c(&thousands(st.total)), c(&format_changes(st)),
        c(&thousands(at.total)), c(&format_changes(at)),
    ];
    table_row(&cells.join(""), ROW_TOTAL_H)
}

fn corps_slide(slide_num: usize, total_slides: usize, c: &CorpsSlide) -> String {
    let (bt, st, at) = sum_kind(&c.today);
    let (by, sy, ay) = sum_kind(&c.yesterday);
    let all_t = bt.total + st.total + at.total;
    let all_y = by.total + sy.total + ay.total;

    let dsk = dsk_label(30 * EMU_CM / 100);
    let pg = page_counter(slide_num, total_slides);

    let section_lbl = text_box(
        120 * EMU_CM / 100, 20 * EMU_CM / 100,
        400 * EMU_CM / 100, 50 * EMU_CM / 100,
        "sec", &text_run("ОГЛЯД ПО ОРГАНАХ", 1000, TEXT_ORANGE, true),
    );

    let title = text_box(
        120 * EMU_CM / 100, 70 * EMU_CM / 100,
        SLIDE_W - 700 * EMU_CM / 100, 100 * EMU_CM / 100,
        "ttl", &text_run(&c.label.to_uppercase(), 2200, TEXT_DARK, true),
    );

    let accent = connector_line(
        120 * EMU_CM / 100, 180 * EMU_CM / 100,
        SLIDE_W - 240 * EMU_CM / 100, 0, TEXT_ORANGE, 19050,
    );

    // Summary tiles
    let tile_y = 230 * EMU_CM / 100;
    let tile_w = 300 * EMU_CM / 100;
    let tile_gap = 320 * EMU_CM / 100;
    let tile_x = 120 * EMU_CM / 100;
    let tile_h = 130 * EMU_CM / 100;

    let mini_tile = |x: i64, label: &str, val: i64, delta: i64| -> String {
        let bg = rect_fill(x, tile_y, tile_w, tile_h, BG_TILE);
        let brd = format!(
            "<p:sp><p:nvSpPr><p:cNvPr id=\"0\" name=\"mb\"/>\
             <p:cNvSpPr/><p:nvPr/></p:nvSpPr>\
             <p:spPr><a:xfrm><a:off x=\"{x}\" y=\"{tile_y}\"/>\
             <a:ext cx=\"{tile_w}\" cy=\"{tile_h}\"/></a:xfrm>\
             <a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom>\
             <a:noFill/>\
             <a:ln w=\"6350\"><a:solidFill><a:srgbClr val=\"{BORDER}\"/></a:solidFill></a:ln>\
             </p:spPr>\
             <p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr/></a:p></p:txBody></p:sp>"
        );
        let top_bar = rect_fill(x, tile_y, tile_w, 12000, TEXT_ORANGE);
        let v = text_box(x + 10 * EMU_CM / 100, tile_y + 10 * EMU_CM / 100,
            tile_w - 20 * EMU_CM / 100, 55 * EMU_CM / 100,
            "mv", &text_run(&thousands(val), 2000, TEXT_ORANGE, true));
        let l = text_box(x + 10 * EMU_CM / 100, tile_y + 60 * EMU_CM / 100,
            tile_w - 20 * EMU_CM / 100, 30 * EMU_CM / 100,
            "ml", &text_run(label, 900, TEXT_DARK, true));
        let d = text_box(x + 10 * EMU_CM / 100, tile_y + 90 * EMU_CM / 100,
            tile_w - 20 * EMU_CM / 100, 30 * EMU_CM / 100,
            "md", &text_run(&format!("зміна: {}", signed_delta(delta)), 800, TEXT_MUTED, false));
        format!("{bg}{top_bar}{brd}{v}{l}{d}")
    };

    let t1 = mini_tile(tile_x, "БЗВП", bt.total, bt.total - by.total);
    let t2 = mini_tile(tile_x + tile_gap, "ФАХОВА", st.total, st.total - sy.total);
    let t3 = mini_tile(tile_x + tile_gap * 2, "АДАПТАЦІЯ", at.total, at.total - ay.total);
    let t4 = mini_tile(tile_x + tile_gap * 3, "ЗАГАЛОМ", all_t, all_t - all_y);

    // Build table
    let rows_data: Vec<&OrgRollupRow> = c.today.main.iter().chain(c.today.out_of_zone.iter()).collect();
    let mut table_rows = String::new();
    table_rows.push_str(&header_row());
    for (i, row) in rows_data.iter().enumerate() {
        table_rows.push_str(&data_row(row, i % 2 == 0));
    }
    table_rows.push_str(&total_row(&bt, &st, &at));

    let n_data_rows = rows_data.len();
    let table_h = ROW_HEADER_H + ROW_DATA_H * n_data_rows as i64 + ROW_TOTAL_H;
    let tw = table_w();
    let tx = (SLIDE_W - tw) / 2;
    let table = table_frame(tx, TABLE_Y, tw, table_h, &col_widths(), &table_rows);

    // Summary text below table
    let summary_y = TABLE_Y + table_h + 60000;
    let fin_total = bt.finishing_today + st.finishing_today + at.finishing_today;
    let start_total = bt.started_today + st.started_today + at.started_today;
    let summary = text_box(
        tx, summary_y, tw, 200000, "sum",
        &text_run(
            &format!(
                "ЗАГАЛОМ ЗА {}:  Залучено {}  \u{00B7}  Завершують {}  \u{00B7}  Розпочинають +{}",
                c.label, thousands(all_t), thousands(fin_total), thousands(start_total),
            ),
            1000, TEXT_AMBER, true,
        ),
    );

    let dsk_bottom = text_box(
        120 * EMU_CM / 100, SLIDE_H - 120 * EMU_CM / 100,
        500 * EMU_CM / 100, 80 * EMU_CM / 100,
        "dskb",
        &text_run("ДЛЯ СЛУЖБОВОГО КОРИСТУВАННЯ", 800, TEXT_MUTED, false),
    );

    wrap_slide(&format!(
        "{dsk}{pg}{section_lbl}{title}{accent}{t1}{t2}{t3}{t4}{table}{summary}{dsk_bottom}"
    ))
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn build_pptx(date: NaiveDate, corps: &[CorpsSlide]) -> Result<Vec<u8>, zip::result::ZipError> {
    let mut total_bt = KindCounts::default();
    let mut total_st = KindCounts::default();
    let mut total_at = KindCounts::default();
    let mut total_by = KindCounts::default();
    let mut total_sy = KindCounts::default();
    let mut total_ay = KindCounts::default();
    for c in corps {
        let (bt, st, at) = sum_kind(&c.today);
        let (by, sy, ay) = sum_kind(&c.yesterday);
        total_bt.total += bt.total; total_bt.finishing_today += bt.finishing_today; total_bt.started_today += bt.started_today;
        total_st.total += st.total; total_st.finishing_today += st.finishing_today; total_st.started_today += st.started_today;
        total_at.total += at.total; total_at.finishing_today += at.finishing_today; total_at.started_today += at.started_today;
        total_by.total += by.total;
        total_sy.total += sy.total;
        total_ay.total += ay.total;
    }

    let total_slides = 2 + corps.len();
    let mut slides = Vec::with_capacity(total_slides);
    slides.push(title_slide(date));
    slides.push(overview_slide(
        date, total_slides,
        total_bt.total, total_by.total,
        total_st.total, total_sy.total,
        total_at.total, total_ay.total,
    ));
    for (i, c) in corps.iter().enumerate() {
        slides.push(corps_slide(i + 3, total_slides, c));
    }

    let n = slides.len();

    let content_types = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/>\
<Override PartName=\"/ppt/presentation.xml\" \
ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml\"/>\
{}</Types>",
        (1..=n)
            .map(|i| format!(
                "<Override PartName=\"/ppt/slides/slide{i}.xml\" \
                 ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/>"
            ))
            .collect::<String>()
    );

    let root_rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" \
Target=\"ppt/presentation.xml\"/></Relationships>";

    let slide_list: String = (1..=n)
        .map(|i| format!("<p:sldId id=\"{}\" r:id=\"rId{i}\"/>", 255 + i))
        .collect();
    let presentation = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<p:presentation xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\">\
<p:sldSz cx=\"{SLIDE_W}\" cy=\"{SLIDE_H}\" type=\"custom\"/>\
<p:notesSz cx=\"{SLIDE_H}\" cy=\"{SLIDE_W}\"/>\
<p:sldIdLst>{slide_list}</p:sldIdLst></p:presentation>"
    );

    let pres_rels = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
{}</Relationships>",
        (1..=n)
            .map(|i| format!(
                "<Relationship Id=\"rId{i}\" \
                 Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" \
                 Target=\"slides/slide{i}.xml\"/>"
            ))
            .collect::<String>()
    );

    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let opts = SimpleFileOptions::default();

    zip.start_file("[Content_Types].xml", opts)?;
    zip.write_all(content_types.as_bytes())?;
    zip.start_file("_rels/.rels", opts)?;
    zip.write_all(root_rels.as_bytes())?;
    zip.start_file("ppt/presentation.xml", opts)?;
    zip.write_all(presentation.as_bytes())?;
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)?;
    zip.write_all(pres_rels.as_bytes())?;

    for (i, xml) in slides.iter().enumerate() {
        zip.start_file(format!("ppt/slides/slide{}.xml", i + 1), opts)?;
        zip.write_all(xml.as_bytes())?;
    }

    Ok(zip.finish()?.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::repo::documents::{DailyRollup, OrgRollupRow};

    fn row(label: &str, bzvp: i64, special: i64) -> OrgRollupRow {
        let kc = |total| KindCounts { total, finishing_today: 1, started_today: 2, left_today: 0 };
        OrgRollupRow {
            org_id: 1,
            org_label: label.into(),
            bzvp: kc(bzvp),
            special: kc(special),
            adaptation: kc(0),
            note: None,
        }
    }

    #[test]
    fn pptx_is_valid_zip_with_slides() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 26).unwrap();
        let corps = [
            CorpsSlide {
                label: "17 АК".into(),
                today: DailyRollup { main: vec![row("ч1", 100, 50)], out_of_zone: vec![] },
                yesterday: DailyRollup { main: vec![row("ч1", 90, 45)], out_of_zone: vec![] },
            },
            CorpsSlide {
                label: "30 КМП".into(),
                today: DailyRollup { main: vec![row("ч2", 200, 80)], out_of_zone: vec![] },
                yesterday: DailyRollup { main: vec![row("ч2", 180, 70)], out_of_zone: vec![] },
            },
        ];
        let bytes = build_pptx(date, &corps).unwrap();
        let archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        // title + overview + 2 corps = 4 slides
        assert!(archive.len() >= 6);
        let names: Vec<_> = archive.file_names().collect();
        assert!(names.contains(&"ppt/slides/slide1.xml"));
        assert!(names.contains(&"ppt/slides/slide2.xml"));
        assert!(names.contains(&"ppt/slides/slide3.xml"));
        assert!(names.contains(&"ppt/slides/slide4.xml"));
    }

    #[test]
    fn slide_content_has_date_and_corps() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 26).unwrap();
        let corps = [CorpsSlide {
            label: "17 АК".into(),
            today: DailyRollup { main: vec![row("ч1", 100, 0)], out_of_zone: vec![] },
            yesterday: DailyRollup { main: vec![row("ч1", 90, 0)], out_of_zone: vec![] },
        }];
        let bytes = build_pptx(date, &corps).unwrap();
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut xml = String::new();
        std::io::Read::read_to_string(
            &mut archive.by_name("ppt/slides/slide1.xml").unwrap(),
            &mut xml,
        )
        .unwrap();
        assert!(xml.contains("26.09.2026"));
        // slide3 = first corps slide (title, overview, then corps)
        let mut xml3 = String::new();
        std::io::Read::read_to_string(
            &mut archive.by_name("ppt/slides/slide3.xml").unwrap(),
            &mut xml3,
        )
        .unwrap();
        assert!(xml3.contains("17 АК"));
    }

    #[test]
    fn corps_slide_has_table_element() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 26).unwrap();
        let corps = [CorpsSlide {
            label: "20 АК".into(),
            today: DailyRollup {
                main: vec![row("23 омбр", 200, 80), row("31 омбр", 150, 60)],
                out_of_zone: vec![],
            },
            yesterday: DailyRollup {
                main: vec![row("23 омбр", 190, 75), row("31 омбр", 140, 55)],
                out_of_zone: vec![],
            },
        }];
        let bytes = build_pptx(date, &corps).unwrap();
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut xml = String::new();
        std::io::Read::read_to_string(
            &mut archive.by_name("ppt/slides/slide3.xml").unwrap(),
            &mut xml,
        )
        .unwrap();
        assert!(xml.contains("<a:tbl>"), "slide must contain a real table");
        assert!(xml.contains("23 омбр"), "table must contain unit names");
        assert!(xml.contains("ВСЬОГО"), "table must contain totals row");
    }

    #[test]
    fn format_changes_cases() {
        let both = KindCounts { total: 100, finishing_today: 5, started_today: 3, left_today: 0 };
        assert_eq!(format_changes(&both), "5/+3");
        let fin_only = KindCounts { total: 100, finishing_today: 12, started_today: 0, left_today: 0 };
        assert_eq!(format_changes(&fin_only), "12");
        let start_only = KindCounts { total: 100, finishing_today: 0, started_today: 7, left_today: 0 };
        assert_eq!(format_changes(&start_only), "+7");
        let none = KindCounts { total: 100, finishing_today: 0, started_today: 0, left_today: 0 };
        assert_eq!(format_changes(&none), "\u{2014}");
    }
}
