//! Генератор CSS (08 §9) — окремий явний крок, запускається ВРУЧНУ ПЕРЕД `cargo leptos build`
//! (`cargo run -p style --bin gen`), не `build.rs` (не встигає — style-крок `cargo-leptos`
//! паралельний і незалежний від збірки Rust-крейта, підтверджено емпірично) і не `@import` (не
//! розгортається Lightning CSS-кроком цього проєкту).
//!
//! Дописує/перезаписує ОДИН помічений блок у кінці `app/style/main.css` (між маркерами) — решта
//! файлу (існуючий ручний BEM-CSS, Фаза 3 його поступово замінить) лишається недоторканою.
//! Ідемпотентно: повторний запуск дає той самий результат, не накопичує дублікати.
//!
//! Заразом перезаписує `style/ATOMS.md` (шпаргалка для агента) — та сама команда, той самий
//! момент, обидва артефакти завжди в парі з `grammar_data.rs`.

const START_MARKER: &str =
    "/* === style::generate_css() — НЕ РЕДАГУВАТИ ВРУЧНУ, cargo run -p style --bin gen === */";
const END_MARKER: &str = "/* === кінець style::generate_css() === */";

fn main() {
    // CARGO_MANIFEST_DIR — завжди тека style/ (де лежить style/Cargo.toml), незалежно від того,
    // звідки реально запущено `cargo run` (кореня workspace чи будь-де) — надійніше за
    // відносний шлях від поточної робочої директорії.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../app/style/main.css");
    let existing = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("не вдалось прочитати {}: {e}", path.display());
        std::process::exit(1);
    });

    let before = match existing.find(START_MARKER) {
        Some(idx) => existing[..idx].trim_end(),
        None => existing.trim_end(),
    };

    let generated = style::generate_css();
    let new_content = format!("{before}\n\n{START_MARKER}\n{generated}\n{END_MARKER}\n");

    std::fs::write(&path, &new_content).unwrap_or_else(|e| {
        eprintln!("не вдалось записати {}: {e}", path.display());
        std::process::exit(1);
    });

    println!(
        "записано в {}: {} байт згенерованого CSS (+ {} байт наявного вручну)",
        path.display(),
        generated.len(),
        before.len()
    );

    let atoms_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ATOMS.md");
    let atoms_md = style::generate_atoms_md();
    std::fs::write(&atoms_path, &atoms_md).unwrap_or_else(|e| {
        eprintln!("не вдалось записати {}: {e}", atoms_path.display());
        std::process::exit(1);
    });
    println!(
        "записано в {}: {} байт",
        atoms_path.display(),
        atoms_md.len()
    );
}
