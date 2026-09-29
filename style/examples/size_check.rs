//! Заміряє розмір `generate_css()` — некомпресований і gzip — щоб вирішити, чи "повний набір
//! атомів" (08 §9) виправдовує складність сканера "лише використані класи", чи ні (Фаза 1).
use std::io::Write;

fn main() {
    let css = style::generate_css();
    let raw = css.len();

    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(css.as_bytes()).unwrap();
    let gz = encoder.finish().unwrap().len();

    println!("рядків CSS: {}", css.lines().count());
    println!("некомпресовано: {raw} байт ({:.1} КБ)", raw as f64 / 1024.0);
    println!("gzip: {gz} байт ({:.1} КБ)", gz as f64 / 1024.0);
}
