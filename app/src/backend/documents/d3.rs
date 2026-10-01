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
Станом на {{date}} у підготовці: БЗВП — {{bzvp_total}} ({{bzvp_delta}} за добу, {{bzvp_yesterday}} → {{bzvp_total}}), \
фахова — {{special_total}} ({{special_delta}} за добу, {{special_yesterday}} → {{special_total}}), \
адаптація — {{adaptation_total}} ({{adaptation_delta}} за добу, {{adaptation_yesterday}} → {{adaptation_total}}).
Загалом залучено {{all_total}} ({{all_delta}} за добу). Рейтинг органів за часткою: {{ranking}}.
# Слайд 2. Огляд
Фахова підготовка: {{special_leaders}}.
Адаптація: {{adaptation_leaders}}.
БЗВП: {{bzvp_leaders}}.
# Далі — по органах
@corps {{corps}}: БЗВП — {{bzvp_total}} (завершують {{bzvp_finishing}}, розпочинають {{bzvp_started}}); \
фахова — {{special_total}} (завершують {{special_finishing}}, розпочинають {{special_started}}); \
адаптація — {{adaptation_total}} (завершують {{adaptation_finishing}}, розпочинають {{adaptation_started}}). \
Загалом {{all_total}}.
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
    for (bold, text) in paragraphs {
        let rpr = if *bold { "<w:rPr><w:b/></w:rPr>" } else { "" };
        body.push_str(&format!(
            "<w:p><w:r>{rpr}<w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
            xml_escape(text)
        ));
    }
    let document = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
<w:body>{body}<w:sectPr><w:pgSz w:w=\"11906\" w:h=\"16838\"/>\
<w:pgMar w:top=\"1134\" w:right=\"850\" w:bottom=\"1134\" w:left=\"1701\" w:header=\"709\" w:footer=\"709\" w:gutter=\"0\"/>\
</w:sectPr></w:body></w:document>"
    );

    const CONTENT_TYPES: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/>\
<Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/>\
</Types>";
    const ROOT_RELS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/>\
</Relationships>";

    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let opts = SimpleFileOptions::default();
    for (name, content) in [
        ("[Content_Types].xml", CONTENT_TYPES),
        ("_rels/.rels", ROOT_RELS),
        ("word/document.xml", document.as_str()),
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
        assert!(xml.contains("<w:b/>"));
    }
}
