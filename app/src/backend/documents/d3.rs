//! D3 "Говорілка" (05 §D3) — текст доповіді в docx: абзаци по слайдах презентації D4.
//!
//! Текст — шаблон з підстановками `{{ім'я}}`; абзац, що починається з `@corps `, повторюється для
//! кожного корпусу (з його значеннями), абзац з `# ` — жирний заголовок слайда. Вбудований
//! шаблон — `DEFAULT_TEMPLATE`; адмін може підмінити його текстовим файлом (`DOCUMENTS_D3_TEMPLATE`,
//! читає `pages/documents/server.rs`). Еталон `Говорілка_26.09.2026.docx` недоступний на dev-VM
//! (`source_files/`), тож дефолтне формулювання — за описом 05 §D3, не посимвольна копія.
//!
//! Зміна за добу = всього на дату мінус всього на попередню дату. Людей поіменно тут немає —
//! лише кількості (CLAUDE.md, жорсткі правила).

use std::collections::HashMap;
use std::io::{Cursor, Write};

use chrono::NaiveDate;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::backend::repo::documents::{DailyRollup, KindCounts};
use crate::domain::format::{percent, signed_delta, thousands};

pub const DEFAULT_TEMPLATE: &str = "\
# Слайд 1. Загальна динаміка
Станом на {{date}} у підготовці: {{summary_kinds}}.
Загалом залучено {{all_total}} ({{all_delta}} за добу). Рейтинг органів за часткою: {{ranking}}.
# Слайд 2. Огляд
{{overview_kinds}}
# Далі — по органах
@corps {{corps}}: {{corps_kinds}}. Загалом {{all_total}}.
";

/// Дані одного корпусу: rollup на дату й на попередню дату (для зміни за добу).
pub struct CorpsDay {
    pub label: String,
    pub today: DailyRollup,
    pub yesterday: DailyRollup,
}

#[derive(Clone, Copy, Default)]
struct Sums {
    bzvp: KindCounts,
    special: KindCounts,
    adaptation: KindCounts,
}

impl Sums {
    fn of(rollup: &DailyRollup) -> Self {
        let mut s = Sums::default();
        for row in rollup.main.iter().chain(rollup.out_of_zone.iter()) {
            s.merge(&Sums { bzvp: row.bzvp, special: row.special, adaptation: row.adaptation });
        }
        s
    }

    fn merge(&mut self, other: &Sums) {
        for (acc, add) in [
            (&mut self.bzvp, other.bzvp),
            (&mut self.special, other.special),
            (&mut self.adaptation, other.adaptation),
        ] {
            acc.total += add.total;
            acc.finishing_today += add.finishing_today;
            acc.started_today += add.started_today;
            acc.left_today += add.left_today;
        }
    }

    fn all_total(&self) -> i64 {
        self.bzvp.total + self.special.total + self.adaptation.total
    }
}

fn kind_of(s: &Sums, key: &str) -> KindCounts {
    match key {
        "bzvp" => s.bzvp,
        "special" => s.special,
        _ => s.adaptation,
    }
}

const KINDS: [&str; 3] = ["bzvp", "special", "adaptation"];

fn kind_name_ua(key: &str) -> &'static str {
    match key {
        "bzvp" => "БЗВП",
        "special" => "фахова",
        _ => "адаптація",
    }
}

fn fill_scope(vars: &mut HashMap<String, String>, today: &Sums, yesterday: &Sums) {
    for key in KINDS {
        let t = kind_of(today, key);
        let y = kind_of(yesterday, key);
        vars.insert(format!("{key}_total"), thousands(t.total));
        vars.insert(format!("{key}_yesterday"), thousands(y.total));
        vars.insert(format!("{key}_delta"), signed_delta(t.total - y.total));
        vars.insert(format!("{key}_finishing"), thousands(t.finishing_today));
        vars.insert(format!("{key}_started"), thousands(t.started_today));
    }
    vars.insert("all_total".into(), thousands(today.all_total()));
    vars.insert("all_delta".into(), signed_delta(today.all_total() - yesterday.all_total()));

    let mut parts = Vec::new();
    for key in KINDS {
        let t = kind_of(today, key);
        let y = kind_of(yesterday, key);
        if t.total == 0 && y.total == 0 { continue; }
        parts.push(format!(
            "{} — {} ({} за добу, {} → {})",
            kind_name_ua(key), thousands(t.total), signed_delta(t.total - y.total),
            thousands(y.total), thousands(t.total),
        ));
    }
    vars.insert("summary_kinds".into(), if parts.is_empty() { "немає даних".into() } else { parts.join(", ") });

    let mut corps_parts = Vec::new();
    for key in KINDS {
        let t = kind_of(today, key);
        if t.total == 0 && t.finishing_today == 0 && t.started_today == 0 { continue; }
        corps_parts.push(format!(
            "{} — {} (завершують {}, розпочинають {})",
            kind_name_ua(key), thousands(t.total),
            thousands(t.finishing_today), thousands(t.started_today),
        ));
    }
    vars.insert("corps_kinds".into(), if corps_parts.is_empty() { "немає даних".into() } else { corps_parts.join("; ") });
}

fn fill_overview(vars: &mut HashMap<String, String>, today: &Sums, per_corps: &[(&CorpsDay, Sums, Sums)]) {
    let mut lines = Vec::new();
    for key in KINDS {
        let t = kind_of(today, key);
        if t.total == 0 { continue; }
        let name = match key {
            "bzvp" => "БЗВП",
            "special" => "Фахова підготовка",
            _ => "Адаптація",
        };
        let shares: Vec<(String, i64)> = per_corps
            .iter()
            .map(|(c, ct, _)| (c.label.clone(), kind_of(ct, key).total))
            .collect();
        let ldrs = leaders(&shares, t.total);
        lines.push(format!("{name}: {ldrs}."));
    }
    vars.insert("overview_kinds".into(), if lines.is_empty() { "Немає даних для огляду.".into() } else { lines.join("\n") });
}

/// "ОТУ «Одеса» — 41%, 30 КМП — 22%, …" — топ-4 за часткою; корпуси з нулем пропускаються.
fn leaders(corps: &[(String, i64)], whole: i64) -> String {
    let mut ranked: Vec<&(String, i64)> = corps.iter().filter(|(_, n)| *n > 0).collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    if ranked.is_empty() {
        return "жодного залучення".to_string();
    }
    ranked
        .iter()
        .take(4)
        .map(|(label, n)| format!("{label} — {}", percent(*n, whole)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn substitute(line: &str, vars: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find("}}") {
            Some(end) => {
                let name = after[..end].trim();
                match vars.get(name) {
                    Some(v) => out.push_str(v),
                    None => {
                        out.push_str("{{");
                        out.push_str(&after[..end]);
                        out.push_str("}}");
                    }
                }
                rest = &after[end + 2..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// Абзаци результату: `(жирний, текст)`.
pub fn render_paragraphs(
    template: &str,
    date: NaiveDate,
    corps: &[CorpsDay],
) -> Vec<(bool, String)> {
    let per_corps: Vec<(&CorpsDay, Sums, Sums)> =
        corps.iter().map(|c| (c, Sums::of(&c.today), Sums::of(&c.yesterday))).collect();

    let mut total_today = Sums::default();
    let mut total_yesterday = Sums::default();
    for (_, t, y) in &per_corps {
        total_today.merge(t);
        total_yesterday.merge(y);
    }

    let mut globals = HashMap::new();
    globals.insert("date".to_string(), date.format("%d.%m.%Y").to_string());
    fill_scope(&mut globals, &total_today, &total_yesterday);
    fill_overview(&mut globals, &total_today, &per_corps);
    for key in KINDS {
        let shares: Vec<(String, i64)> = per_corps
            .iter()
            .map(|(c, t, _)| (c.label.clone(), kind_of(t, key).total))
            .collect();
        globals.insert(format!("{key}_leaders"), leaders(&shares, kind_of(&total_today, key).total));
    }
    let overall: Vec<(String, i64)> =
        per_corps.iter().map(|(c, t, _)| (c.label.clone(), t.all_total())).collect();
    globals.insert("ranking".into(), leaders(&overall, total_today.all_total()));

    let mut out = Vec::new();
    for raw in template.lines() {
        let line = raw.trim_end();
        if line.trim().is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("# ") {
            out.push((true, substitute(rest, &globals)));
        } else if let Some(rest) = line.strip_prefix("@corps ") {
            for (c, t, y) in &per_corps {
                if t.all_total() == 0 && y.all_total() == 0 {
                    continue;
                }
                let mut vars = globals.clone();
                vars.insert("corps".into(), c.label.clone());
                fill_scope(&mut vars, t, y);
                out.push((false, substitute(rest, &vars)));
            }
        } else {
            out.push((false, substitute(line, &globals)));
        }
    }
    out
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

pub fn build_docx(paragraphs: &[(bool, String)]) -> Result<Vec<u8>, zip::result::ZipError> {
    let mut body = String::new();

    // Title block: "ДОПОВІДЬ" with orange accent bar
    body.push_str(
        "<w:p><w:pPr><w:pStyle w:val=\"Title\"/></w:pPr>\
         <w:r><w:rPr/><w:t>ДОПОВІДЬ</w:t></w:r></w:p>"
    );
    body.push_str(
        "<w:p><w:pPr><w:pStyle w:val=\"Subtitle\"/></w:pPr>\
         <w:r><w:rPr/><w:t>Угруповання військ (сил) \"Південь\" — динаміка підготовки</w:t></w:r></w:p>"
    );

    for (bold, text) in paragraphs {
        if *bold {
            body.push_str(&format!(
                "<w:p>\
                 <w:pPr><w:pStyle w:val=\"Heading1\"/></w:pPr>\
                 <w:r><w:rPr/>\
                 <w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
                xml_escape(text)
            ));
        } else {
            body.push_str(&format!(
                "<w:p>\
                 <w:pPr><w:pStyle w:val=\"Normal\"/></w:pPr>\
                 <w:r><w:rPr/>\
                 <w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
                xml_escape(text)
            ));
        }
    }

    let document = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\
<w:body>{body}<w:sectPr>\
<w:headerReference w:type=\"default\" r:id=\"rId2\"/>\
<w:footerReference w:type=\"default\" r:id=\"rId3\"/>\
<w:pgSz w:w=\"11906\" w:h=\"16838\"/>\
<w:pgMar w:top=\"1418\" w:right=\"850\" w:bottom=\"1134\" w:left=\"1701\" w:header=\"567\" w:footer=\"567\" w:gutter=\"0\"/>\
</w:sectPr></w:body></w:document>"
    );

    const STYLES: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
<w:docDefaults><w:rPrDefault><w:rPr>\
<w:rFonts w:ascii=\"Calibri\" w:hAnsi=\"Calibri\" w:cs=\"Arial\"/>\
<w:sz w:val=\"22\"/><w:szCs w:val=\"22\"/>\
<w:color w:val=\"2A2620\"/>\
</w:rPr></w:rPrDefault>\
<w:pPrDefault><w:pPr>\
<w:spacing w:after=\"100\" w:line=\"288\" w:lineRule=\"auto\"/>\
</w:pPr></w:pPrDefault>\
</w:docDefaults>\
<w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\">\
<w:name w:val=\"Normal\"/>\
<w:pPr><w:spacing w:after=\"120\" w:line=\"288\" w:lineRule=\"auto\"/></w:pPr>\
<w:rPr><w:rFonts w:ascii=\"Calibri\" w:hAnsi=\"Calibri\"/>\
<w:sz w:val=\"22\"/><w:color w:val=\"2A2620\"/></w:rPr>\
</w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"Title\">\
<w:name w:val=\"Title\"/>\
<w:pPr>\
<w:spacing w:before=\"0\" w:after=\"40\"/>\
<w:jc w:val=\"center\"/>\
</w:pPr>\
<w:rPr><w:rFonts w:ascii=\"Calibri\" w:hAnsi=\"Calibri\"/>\
<w:b/><w:caps/><w:sz w:val=\"36\"/><w:szCs w:val=\"36\"/>\
<w:color w:val=\"453F2E\"/>\
<w:spacing w:val=\"60\"/></w:rPr>\
</w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"Subtitle\">\
<w:name w:val=\"Subtitle\"/>\
<w:pPr>\
<w:spacing w:before=\"0\" w:after=\"200\"/>\
<w:jc w:val=\"center\"/>\
<w:pBdr><w:bottom w:val=\"single\" w:sz=\"12\" w:space=\"8\" w:color=\"F39200\"/></w:pBdr>\
</w:pPr>\
<w:rPr><w:rFonts w:ascii=\"Calibri\" w:hAnsi=\"Calibri\"/>\
<w:sz w:val=\"22\"/><w:szCs w:val=\"22\"/>\
<w:color w:val=\"7A6F55\"/>\
<w:i/></w:rPr>\
</w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"Heading1\">\
<w:name w:val=\"heading 1\"/>\
<w:pPr>\
<w:spacing w:before=\"320\" w:after=\"120\"/>\
<w:pBdr><w:bottom w:val=\"single\" w:sz=\"8\" w:space=\"4\" w:color=\"E7DDBA\"/></w:pBdr>\
<w:shd w:val=\"clear\" w:color=\"auto\" w:fill=\"F5F2EC\"/>\
<w:ind w:left=\"113\" w:right=\"113\"/>\
</w:pPr>\
<w:rPr><w:rFonts w:ascii=\"Calibri\" w:hAnsi=\"Calibri\"/>\
<w:b/><w:caps/><w:sz w:val=\"26\"/><w:szCs w:val=\"26\"/>\
<w:color w:val=\"453F2E\"/>\
<w:spacing w:val=\"30\"/></w:rPr>\
</w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"Header\">\
<w:name w:val=\"header\"/>\
<w:pPr><w:spacing w:after=\"0\" w:line=\"240\" w:lineRule=\"auto\"/>\
<w:jc w:val=\"right\"/></w:pPr>\
<w:rPr><w:sz w:val=\"16\"/><w:color w:val=\"9C947F\"/></w:rPr>\
</w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"Footer\">\
<w:name w:val=\"footer\"/>\
<w:pPr><w:spacing w:after=\"0\" w:line=\"240\" w:lineRule=\"auto\"/>\
<w:pBdr><w:top w:val=\"single\" w:sz=\"4\" w:space=\"4\" w:color=\"C7C1B0\"/></w:pBdr>\
</w:pPr>\
<w:rPr><w:sz w:val=\"16\"/><w:color w:val=\"9C947F\"/></w:rPr>\
</w:style>\
</w:styles>";

    const HEADER_XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:hdr xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
<w:p><w:pPr><w:pStyle w:val=\"Header\"/></w:pPr>\
<w:r><w:rPr><w:caps/></w:rPr>\
<w:t>Для службового користування</w:t></w:r></w:p>\
</w:hdr>";

    const FOOTER_XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:ftr xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
<w:p><w:pPr><w:pStyle w:val=\"Footer\"/><w:jc w:val=\"center\"/></w:pPr>\
<w:r><w:rPr/><w:t xml:space=\"preserve\">УВ(с) \"Південь\" — </w:t></w:r>\
<w:r><w:rPr/><w:fldChar w:fldCharType=\"begin\"/></w:r>\
<w:r><w:rPr/><w:instrText> PAGE </w:instrText></w:r>\
<w:r><w:rPr/><w:fldChar w:fldCharType=\"end\"/></w:r>\
</w:p></w:ftr>";

    const CONTENT_TYPES: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/>\
<Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/>\
<Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/>\
<Override PartName=\"/word/header1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml\"/>\
<Override PartName=\"/word/footer1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml\"/>\
</Types>";

    const ROOT_RELS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/>\
</Relationships>";

    const DOC_RELS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>\
<Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/header\" Target=\"header1.xml\"/>\
<Relationship Id=\"rId3\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer\" Target=\"footer1.xml\"/>\
</Relationships>";

    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let opts = SimpleFileOptions::default();
    for (name, content) in [
        ("[Content_Types].xml", CONTENT_TYPES),
        ("_rels/.rels", ROOT_RELS),
        ("word/_rels/document.xml.rels", DOC_RELS),
        ("word/document.xml", document.as_str()),
        ("word/styles.xml", STYLES),
        ("word/header1.xml", HEADER_XML),
        ("word/footer1.xml", FOOTER_XML),
    ] {
        zip.start_file(name, opts)?;
        zip.write_all(content.as_bytes())?;
    }
    Ok(zip.finish()?.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::repo::documents::OrgRollupRow;

    fn row(label: &str, bzvp: i64, special: i64, adaptation: i64) -> OrgRollupRow {
        let kc = |total| KindCounts { total, finishing_today: 1, started_today: 2, left_today: 0 };
        OrgRollupRow {
            org_id: 1,
            org_label: label.into(),
            bzvp: kc(bzvp),
            special: kc(special),
            adaptation: kc(adaptation),
            note: None,
        }
    }

    fn corps(label: &str, today: i64, yesterday: i64) -> CorpsDay {
        CorpsDay {
            label: label.into(),
            today: DailyRollup { main: vec![row("ч", today, 0, 0)], out_of_zone: vec![] },
            yesterday: DailyRollup { main: vec![row("ч", yesterday, 0, 0)], out_of_zone: vec![] },
        }
    }

    #[test]
    fn renders_deltas_with_real_minus_and_percent() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 26).unwrap();
        let data = [corps("17 АК", 1500, 1527), corps("30 КМП", 500, 426)];
        let paras = render_paragraphs(DEFAULT_TEMPLATE, date, &data);
        let all = paras.iter().map(|(_, t)| t.as_str()).collect::<Vec<_>>().join("\n");
        assert!(all.contains("26.09.2026"));
        assert!(all.contains("БЗВП — 2\u{A0}000 (+47 за добу"), "{all}");
        assert!(all.contains("17 АК: БЗВП — 1\u{A0}500"), "{all}");
        assert!(all.contains("17 АК — 75%, 30 КМП — 25%"), "{all}");
        assert!(!all.contains("{{"), "{all}");
        assert_eq!(paras.iter().filter(|(b, _)| *b).count(), 3);
    }

    #[test]
    fn unknown_placeholder_stays_visible() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 26).unwrap();
        let paras = render_paragraphs("Привіт {{нема}}", date, &[]);
        assert_eq!(paras, vec![(false, "Привіт {{нема}}".to_string())]);
    }

    #[test]
    fn docx_is_valid_zip_with_escaped_text() {
        let bytes = build_docx(&[(true, "А & Б <1>".into())]).unwrap();
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut xml = String::new();
        std::io::Read::read_to_string(&mut archive.by_name("word/document.xml").unwrap(), &mut xml)
            .unwrap();
        assert!(xml.contains("А &amp; Б &lt;1&gt;"));
        assert!(xml.contains("Heading1"));
    }
}
