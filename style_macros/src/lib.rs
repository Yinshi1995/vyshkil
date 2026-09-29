//! `cx!("p4 gap2 fg-muted")` → перевіряє кожен атом-токен проти граматики на етапі КОМПІЛЯЦІЇ,
//! помилка з підказкою найближчого правильного імені (Левенштейн) замість мовчазного тихого
//! рядка з друкарською помилкою, яка ніколи не отримає стилю (07-code-structure "агенту потрібно
//! найменше токенів, щоб правильно стилізувати" — компілятор сам підказує, документація не
//! потрібна для типового випадку).
//!
//! **Фаза-0-обмеження (задокументовано, не приховано)**: `KNOWN_ATOMS` тут — ДУБЛЬОВАНА копія
//! `style::ATOMS`-імен, не читання самого `style` крейту. Proc-macro-крейт МІГ БИ залежати від
//! `style` напряму (немає циклу — `style` не залежить від `style_macros`), але тоді `style::cx!`
//! (реекспорт для ергономіки) вимагав би `style` → `style_macros`, а це і є цикл. Реальне рішення
//! (єдине джерело граматики для обох крейтів без циклу залежностей) — Фаза 1: спільний
//! дата-файл (`include!`) чи третій крейт `style_core`, який тримає лише дані. Прототип свідомо
//! не вирішує це — 20 атомів синхронизувати вручну прийнятно, 200 — ні.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};

const KNOWN_ATOMS: &[&str] = &[
    "p0", "p1", "p2", "p3", "p4", "gap0", "gap1", "gap2", "gap3", "gap4", "flex", "col",
    "items-c", "justify-b", "fg-main", "fg-muted", "fg-accent", "bg-panel", "r0", "r1",
];

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut prev = row[0];
        row[0] = i;
        for j in 1..=b.len() {
            let tmp = row[j];
            row[j] =
                if a[i - 1] == b[j - 1] { prev } else { 1 + prev.min(row[j]).min(row[j - 1]) };
            prev = tmp;
        }
    }
    row[b.len()]
}

fn closest(unknown: &str) -> Option<&'static str> {
    KNOWN_ATOMS.iter().map(|a| (*a, levenshtein(unknown, a))).min_by_key(|(_, d)| *d).map(|(a, _)| a)
}

/// Валідує рядок атомів, розділених пробілами, і повертає його як `&'static str` (сам рядок —
/// водночас список CSS-класів, бо ім'я атома == ім'я класу в `style::generate_css`).
#[proc_macro]
pub fn cx(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let value = lit.value();

    let mut unknown_atoms = Vec::new();
    for atom in value.split_whitespace() {
        if !KNOWN_ATOMS.contains(&atom) {
            unknown_atoms.push(atom.to_string());
        }
    }

    if !unknown_atoms.is_empty() {
        let messages: Vec<String> = unknown_atoms
            .iter()
            .map(|atom| match closest(atom) {
                Some(s) => format!("невідомий атом «{atom}» — може, ви мали на увазі «{s}»?"),
                None => format!("невідомий атом «{atom}»"),
            })
            .collect();
        let msg = messages.join("; ");
        return syn::Error::new(lit.span(), msg).to_compile_error().into();
    }

    quote! { #value }.into()
}
