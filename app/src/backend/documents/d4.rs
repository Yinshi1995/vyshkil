//! D4 "Підготовка" (05 §D4) — презентація pptx: титульний слайд + слайд загальної динаміки +
//! по одному слайду на корпус. Генерується zip/XML з нуля (шаблонний підхід із іменованими
//! фігурами потребує `source_files/Зразок/Підготовка_26.09.2026.pptx`, відсутній на dev-VM —
//! docs/QUESTIONS.md). Стиль за 05 §D4: фон `#0E0C08`, панелі `#38331F`, акцент `#F39200`,
//! текст `#F7F5F0`, вторинний `#D2CCBC`. Шрифт UAF Sans (фолбек Arial).
//!
//! Людей поіменно тут немає — лише кількості груп (CLAUDE.md, жорсткі правила).

use std::io::{Cursor, Write};

use chrono::NaiveDate;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::backend::repo::documents::{DailyRollup, KindCounts};
use crate::domain::format::{signed_delta, thousands};

pub struct CorpsSlide {
    pub label: String,
    pub today: DailyRollup,
    pub yesterday: DailyRollup,
}

const EMU_CM: i64 = 360000;
const SLIDE_W: i64 = 33_87 * EMU_CM / 100;
const SLIDE_H: i64 = 19_05 * EMU_CM / 100;

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

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn text_run(text: &str, font_size: u32, color: &str, bold: bool) -> String {
    let b = if bold { "<a:rPr b=\"1\"/>" } else { "" };
    format!(
        "<a:r><a:rPr lang=\"uk-UA\" sz=\"{font_size}\" dirty=\"0\">\
         <a:solidFill><a:srgbClr val=\"{color}\"/></a:solidFill>\
         <a:latin typeface=\"UAF Sans\" panose=\"020B0604020202020204\"/>\
         <a:cs typeface=\"Arial\"/>{b}</a:rPr>\
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

fn kpi_tile(x: i64, y: i64, label: &str, value: &str, sub: &str) -> String {
    let w = 350 * EMU_CM / 100;
    let h = 200 * EMU_CM / 100;
    let bg = format!(
        "<p:sp><p:nvSpPr><p:cNvPr id=\"0\" name=\"bg-{label}\"/>\
         <p:cNvSpPr/><p:nvPr/></p:nvSpPr>\
         <p:spPr><a:xfrm><a:off x=\"{x}\" y=\"{y}\"/>\
         <a:ext cx=\"{w}\" cy=\"{h}\"/></a:xfrm>\
         <a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom>\
         <a:solidFill><a:srgbClr val=\"38331F\"/></a:solidFill></p:spPr>\
         <p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr/></a:p></p:txBody></p:sp>"
    );
    let num = text_box(
        x + 20 * EMU_CM / 100,
        y + 15 * EMU_CM / 100,
        w - 40 * EMU_CM / 100,
        80 * EMU_CM / 100,
        &format!("val-{label}"),
        &text_run(value, 3600, "F39200", true),
    );
    let lbl = text_box(
        x + 20 * EMU_CM / 100,
        y + 95 * EMU_CM / 100,
        w - 40 * EMU_CM / 100,
        50 * EMU_CM / 100,
        &format!("lbl-{label}"),
        &text_run(label, 1200, "F7F5F0", false),
    );
    let detail = text_box(
        x + 20 * EMU_CM / 100,
        y + 140 * EMU_CM / 100,
        w - 40 * EMU_CM / 100,
        50 * EMU_CM / 100,
        &format!("sub-{label}"),
        &text_run(sub, 1000, "D2CCBC", false),
    );
    format!("{bg}{num}{lbl}{detail}")
}

fn slide_bg() -> &'static str {
    "<p:bg><p:bgPr><a:solidFill><a:srgbClr val=\"0E0C08\"/></a:solidFill>\
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

fn title_slide(date: NaiveDate) -> String {
    let title = text_box(
        200 * EMU_CM / 100,
        600 * EMU_CM / 100,
        SLIDE_W - 400 * EMU_CM / 100,
        300 * EMU_CM / 100,
        "title",
        &text_run("УГРУПОВАННЯ ВІЙСЬК (СИЛ) \"ПІВДЕНЬ\"", 2800, "F7F5F0", true),
    );
    let sub = text_box(
        200 * EMU_CM / 100,
        950 * EMU_CM / 100,
        SLIDE_W - 400 * EMU_CM / 100,
        200 * EMU_CM / 100,
        "subtitle",
        &text_run(
            &format!("ПІДГОТОВКА станом на {}", date.format("%d.%m.%Y")),
            2000,
            "F39200",
            false,
        ),
    );
    let dsk = text_box(
        200 * EMU_CM / 100,
        SLIDE_H - 150 * EMU_CM / 100,
        SLIDE_W - 400 * EMU_CM / 100,
        100 * EMU_CM / 100,
        "dsk",
        &text_run("ДЛЯ СЛУЖБОВОГО КОРИСТУВАННЯ", 800, "A8A08A", false),
    );
    wrap_slide(&format!("{title}{sub}{dsk}"))
}

fn overview_slide(
    date: NaiveDate,
    bzvp_t: i64,
    bzvp_y: i64,
    special_t: i64,
    special_y: i64,
    adapt_t: i64,
    adapt_y: i64,
) -> String {
    let header = text_box(
        100 * EMU_CM / 100,
        50 * EMU_CM / 100,
        SLIDE_W - 200 * EMU_CM / 100,
        120 * EMU_CM / 100,
        "header",
        &text_run(
            &format!("ЗАГАЛЬНА ДИНАМІКА ПІДГОТОВКИ станом на {}", date.format("%d.%m.%Y")),
            1800,
            "F39200",
            true,
        ),
    );

    let left = 100 * EMU_CM / 100;
    let gap = 380 * EMU_CM / 100;
    let top = 250 * EMU_CM / 100;

    let bzvp = kpi_tile(
        left,
        top,
        "БЗВП",
        &thousands(bzvp_t),
        &format!("зміна за добу: {}", signed_delta(bzvp_t - bzvp_y)),
    );
    let special = kpi_tile(
        left + gap,
        top,
        "ФАХОВА",
        &thousands(special_t),
        &format!("зміна за добу: {}", signed_delta(special_t - special_y)),
    );
    let adapt = kpi_tile(
        left + gap * 2,
        top,
        "АДАПТАЦІЯ",
        &thousands(adapt_t),
        &format!("зміна за добу: {}", signed_delta(adapt_t - adapt_y)),
    );

    let total_t = bzvp_t + special_t + adapt_t;
    let total_y = bzvp_y + special_y + adapt_y;
    let total = kpi_tile(
        left + gap * 3,
        top,
        "ЗАГАЛОМ",
        &thousands(total_t),
        &format!("зміна: {}", signed_delta(total_t - total_y)),
    );

    let dsk = text_box(
        200 * EMU_CM / 100,
        SLIDE_H - 150 * EMU_CM / 100,
        SLIDE_W - 400 * EMU_CM / 100,
        100 * EMU_CM / 100,
        "dsk",
        &text_run("ДЛЯ СЛУЖБОВОГО КОРИСТУВАННЯ", 800, "A8A08A", false),
    );

    wrap_slide(&format!("{header}{bzvp}{special}{adapt}{total}{dsk}"))
}

fn corps_slide(slide_num: usize, c: &CorpsSlide) -> String {
    let (bt, st, at) = sum_kind(&c.today);
    let (by, sy, ay) = sum_kind(&c.yesterday);

    let header = text_box(
        100 * EMU_CM / 100,
        50 * EMU_CM / 100,
        SLIDE_W - 300 * EMU_CM / 100,
        120 * EMU_CM / 100,
        "header",
        &text_run(&c.label.to_uppercase(), 2000, "F39200", true),
    );
    let num = text_box(
        SLIDE_W - 200 * EMU_CM / 100,
        50 * EMU_CM / 100,
        150 * EMU_CM / 100,
        80 * EMU_CM / 100,
        "slidenum",
        &text_run(&slide_num.to_string(), 1400, "F39200", true),
    );

    let left = 100 * EMU_CM / 100;
    let gap = 380 * EMU_CM / 100;
    let top = 250 * EMU_CM / 100;
    let bzvp = kpi_tile(
        left,
        top,
        "БЗВП",
        &thousands(bt.total),
        &format!(
            "завершують {}, розпочинають {}",
            thousands(bt.finishing_today),
            thousands(bt.started_today)
        ),
    );
    let special = kpi_tile(
        left + gap,
        top,
        "ФАХОВА",
        &thousands(st.total),
        &format!(
            "завершують {}, розпочинають {}",
            thousands(st.finishing_today),
            thousands(st.started_today)
        ),
    );
    let adapt = kpi_tile(
        left + gap * 2,
        top,
        "АДАПТАЦІЯ",
        &thousands(at.total),
        &format!(
            "завершують {}, розпочинають {}",
            thousands(at.finishing_today),
            thousands(at.started_today)
        ),
    );
    let total = kpi_tile(
        left + gap * 3,
        top,
        "ЗАГАЛОМ",
        &thousands(bt.total + st.total + at.total),
        &format!(
            "зміна: {}",
            signed_delta(
                (bt.total + st.total + at.total) - (by.total + sy.total + ay.total)
            )
        ),
    );

    let mut table_lines = Vec::new();
    for row in c.today.main.iter().chain(c.today.out_of_zone.iter()) {
        let all = row.bzvp.total + row.special.total + row.adaptation.total;
        table_lines.push(format!(
            "{}  |  БЗВП {}  |  Фах {}  |  Адапт {}  |  Разом {}",
            row.org_label,
            thousands(row.bzvp.total),
            thousands(row.special.total),
            thousands(row.adaptation.total),
            thousands(all)
        ));
    }
    let table_text = table_lines.join("\n");
    let table = text_box(
        100 * EMU_CM / 100,
        500 * EMU_CM / 100,
        SLIDE_W - 200 * EMU_CM / 100,
        SLIDE_H - 650 * EMU_CM / 100,
        "table",
        &text_run(&table_text, 1000, "D2CCBC", false),
    );

    let dsk = text_box(
        200 * EMU_CM / 100,
        SLIDE_H - 150 * EMU_CM / 100,
        SLIDE_W - 400 * EMU_CM / 100,
        100 * EMU_CM / 100,
        "dsk",
        &text_run("ДЛЯ СЛУЖБОВОГО КОРИСТУВАННЯ", 800, "A8A08A", false),
    );

    wrap_slide(&format!("{header}{num}{bzvp}{special}{adapt}{total}{table}{dsk}"))
}

pub fn build_pptx(date: NaiveDate, corps: &[CorpsSlide]) -> Result<Vec<u8>, zip::result::ZipError> {
    let mut slides = Vec::with_capacity(2 + corps.len());
    slides.push(title_slide(date));

    let mut total_bt = KindCounts::default();
    let mut total_st = KindCounts::default();
    let mut total_at = KindCounts::default();
    let mut total_by = KindCounts::default();
    let mut total_sy = KindCounts::default();
    let mut total_ay = KindCounts::default();
    for c in corps {
        let (bt, st, at) = sum_kind(&c.today);
        let (by, sy, ay) = sum_kind(&c.yesterday);
        total_bt.total += bt.total;
        total_st.total += st.total;
        total_at.total += at.total;
        total_by.total += by.total;
        total_sy.total += sy.total;
        total_ay.total += ay.total;
    }
    slides.push(overview_slide(
        date,
        total_bt.total,
        total_by.total,
        total_st.total,
        total_sy.total,
        total_at.total,
        total_ay.total,
    ));

    for (i, c) in corps.iter().enumerate() {
        slides.push(corps_slide(i + 3, c));
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
        let mut xml3 = String::new();
        std::io::Read::read_to_string(
            &mut archive.by_name("ppt/slides/slide3.xml").unwrap(),
            &mut xml3,
        )
        .unwrap();
        assert!(xml3.contains("17 АК"));
    }
}
