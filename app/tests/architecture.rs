//! Архітектурний тест (docs/spec/07-code-structure.md §4) — читає `app/src` через `std::fs`,
//! без компіляції й без БД, тому працює в звичайному `cargo test` (без `--features ssr`).
//! Кожне порушення друкується окремим, людським рядком: що саме не так і як виправити.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Всі `.rs`-файли під `app/src` (рекурсивно), крім `mod.rs`/`lib.rs` — для перевірок
/// "leptos/sea_orm не імпортовано" і "де лежать #[server]".
fn all_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("не читається {dir:?}: {e}")) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            all_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Директорії під `app/src` (рекурсивно, включно з коренем).
fn all_dirs(dir: &Path, out: &mut Vec<PathBuf>) {
    out.push(dir.to_path_buf());
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            all_dirs(&path, out);
        }
    }
}

/// Прямі елементи теки (файли + підтеки), без `mod.rs`/`lib.rs`/`CLAUDE.md` — те, що має бути
/// згадане в карті цієй теки.
fn direct_children_excluding_mod(dir: &Path) -> Vec<String> {
    let mut names = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name == "mod.rs" || name == "lib.rs" || name == "CLAUDE.md" {
            continue;
        }
        if path.is_dir() {
            names.push(format!("{name}/"));
        } else if name.ends_with(".rs") {
            names.push(name);
        }
    }
    names
}

/// Кількість прямих елементів теки, включно з `mod.rs` — поріг "≥2 елементи" з 07 §4.
fn total_direct_children(dir: &Path) -> usize {
    fs::read_dir(dir)
        .unwrap()
        .filter(|e| e.as_ref().unwrap().file_name() != "CLAUDE.md")
        .count()
}

/// Перша комірка кожного рядка markdown-таблиці ("| `x.rs` | ... | ... |") — саме тут карта
/// стверджує "цей шлях існує в цій теці"; інші згадки бектиків (у розділі правил/прози) —
/// ілюстрація патерна ("`pages/<p>/server.rs`"), а не твердження про конкретний файл.
fn table_first_cells(map: &str) -> Vec<String> {
    map.lines()
        .filter(|l| l.trim_start().starts_with('|'))
        .filter_map(|l| l.split('`').nth(1).map(str::to_string))
        .filter(|cell| !cell.contains('<')) // "Елемент"/заголовок або патерн з плейсхолдером
        .collect()
}

fn display_rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

#[test]
fn every_dir_with_two_or_more_elements_has_an_up_to_date_map() {
    let root = src_dir();
    let mut dirs = Vec::new();
    all_dirs(&root, &mut dirs);

    let mut violations = Vec::new();

    for dir in &dirs {
        let rel = display_rel(&root, dir);
        let map_path = dir.join("CLAUDE.md");
        let needs_map = total_direct_children(dir) >= 2;

        if needs_map && !map_path.exists() {
            violations.push(format!(
                "app/src/{rel} має ≥2 елементи, але без CLAUDE.md — додай карту (07 §4)"
            ));
            continue;
        }
        if !map_path.exists() {
            continue;
        }

        let map = fs::read_to_string(&map_path).unwrap();

        if map.lines().count() > 40 {
            violations.push(format!(
                "app/src/{rel}/CLAUDE.md довша за 40 рядків — тека завелика, дели її (07 §4)"
            ));
        }

        for child in direct_children_excluding_mod(dir) {
            let token = format!("`{child}`");
            if !map.contains(&token) {
                violations.push(format!(
                    "app/src/{rel}/{child} не згаданий у app/src/{rel}/CLAUDE.md — онови карту"
                ));
            }
        }

        // Зворотний бік: кожен файл/тека з таблиці карти має реально існувати в цій теці.
        for cell in table_first_cells(&map) {
            if !dir.join(&cell).exists() {
                violations.push(format!(
                    "app/src/{rel}/CLAUDE.md згадує в таблиці `{cell}`, якого нема в цій теці — \
                     онови карту (файл перейменували/перенесли?)"
                ));
            }
        }
    }

    assert!(violations.is_empty(), "\n{}", violations.join("\n"));
}

#[test]
fn no_page_imports_another_page_directly() {
    let pages_dir = src_dir().join("pages");
    if !pages_dir.exists() {
        return;
    }

    let page_names: Vec<String> = fs::read_dir(&pages_dir)
        .unwrap()
        .filter_map(|e| {
            let path = e.unwrap().path();
            path.is_dir().then(|| path.file_name().unwrap().to_string_lossy().to_string())
        })
        .collect();

    let mut violations = Vec::new();

    for page in &page_names {
        let mut files = Vec::new();
        all_rs_files(&pages_dir.join(page), &mut files);

        for file in &files {
            let content = fs::read_to_string(file).unwrap();
            for other in &page_names {
                if other == page {
                    continue;
                }
                let needle = format!("crate::pages::{other}");
                if content.contains(&needle) {
                    violations.push(format!(
                        "{} посилається на {needle} — сторінки не імпортують одна одну (07 §2.2); \
                         спільне перенось у widgets/services (07 §2.1)",
                        file.display()
                    ));
                }
            }
        }
    }

    assert!(violations.is_empty(), "\n{}", violations.join("\n"));
}

#[test]
fn components_does_not_know_the_domain() {
    let components_dir = src_dir().join("components");
    if !components_dir.exists() {
        // Тека ще не існує (07 §1: "з'являється, коли в неї переїжджає перший файл") — ОК.
        return;
    }

    let mut files = Vec::new();
    all_rs_files(&components_dir, &mut files);

    let forbidden = ["crate::services", "crate::backend", "crate::widgets", "crate::pages"];
    let mut violations = Vec::new();

    for file in &files {
        let content = fs::read_to_string(file).unwrap();
        for needle in forbidden {
            if content.contains(needle) {
                violations.push(format!(
                    "{} імпортує {needle} — components/ не знає домену (07 §2.3)",
                    file.display()
                ));
            }
        }
    }

    assert!(violations.is_empty(), "\n{}", violations.join("\n"));
}

#[test]
fn domain_and_types_do_not_import_leptos_or_sea_orm() {
    let mut violations = Vec::new();

    for sub in ["domain", "types"] {
        let dir = src_dir().join(sub);
        if !dir.exists() {
            continue;
        }
        let mut files = Vec::new();
        all_rs_files(&dir, &mut files);

        for file in &files {
            let content = fs::read_to_string(file).unwrap();
            for needle in ["leptos", "sea_orm"] {
                if content.contains(needle) {
                    violations.push(format!(
                        "{} згадує {needle} — {sub}/ мусить лишатись WASM-безпечним, без leptos/sea_orm (07 §2.3)",
                        file.display()
                    ));
                }
            }
        }
    }

    assert!(violations.is_empty(), "\n{}", violations.join("\n"));
}

#[test]
fn backend_is_declared_only_under_ssr_feature() {
    let lib_rs = fs::read_to_string(src_dir().join("lib.rs")).unwrap();
    let backend_line = lib_rs
        .lines()
        .enumerate()
        .find(|(_, l)| l.contains("mod backend"))
        .unwrap_or_else(|| panic!("lib.rs не оголошує `mod backend` — де backend/ реєструється?"));

    let preceding = lib_rs.lines().take(backend_line.0).last().unwrap_or("");
    assert!(
        backend_line.1.contains("cfg(feature = \"ssr\")") || preceding.contains("cfg(feature = \"ssr\")"),
        "lib.rs: `mod backend` не під #[cfg(feature = \"ssr\")] — backend/ використовує sea-orm \
         і не повинен збиратись у WASM (07 §1)"
    );
}

#[test]
fn every_server_fn_lives_in_services_or_a_page_server_file() {
    let root = src_dir();
    let mut files = Vec::new();
    all_rs_files(&root, &mut files);

    let mut violations = Vec::new();

    for file in &files {
        let content = fs::read_to_string(file).unwrap();
        // "#[server(" (з дужкою) — сама атрибут-макро, не згадка "#[server]" у прозі doc-коментаря.
        if !content.contains("#[server(") {
            continue;
        }
        let rel_str = display_rel(&root, file);

        let in_services = rel_str.starts_with("services/");
        let in_page_server = rel_str.starts_with("pages/") && rel_str.ends_with("/server.rs");

        if !in_services && !in_page_server {
            violations.push(format!(
                "app/src/{rel_str} містить #[server], але лежить поза services/ і pages/*/server.rs \
                 (07 §2.4/§3.5)"
            ));
        }
    }

    assert!(violations.is_empty(), "\n{}", violations.join("\n"));
}

/// Карти не дублюють одна одну помилково (той самий шлях згаданий у батьківській карті теки, у
/// якої ВЖЕ є власна карта, — не помилка сама собою, але порожня множина elements підказує, що
/// список тек під контролем; тримаємо як smoke-test, що обхід дерева взагалі щось знайшов).
#[test]
fn sanity_the_tree_walk_actually_found_pages() {
    let pages = BTreeSet::from([
        "dictionaries".to_string(),
        "home".to_string(),
        "import".to_string(),
        "org_detail".to_string(),
        "training_form".to_string(),
        "vos_lookup".to_string(),
    ]);
    let found: BTreeSet<String> = fs::read_dir(src_dir().join("pages"))
        .unwrap()
        .filter_map(|e| {
            let p = e.unwrap().path();
            p.is_dir().then(|| p.file_name().unwrap().to_string_lossy().to_string())
        })
        .collect();
    assert_eq!(found, pages, "очікував саме ці сторінки — онови цей тест, якщо додав нову");
}
