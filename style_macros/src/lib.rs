//! `cx!("p4 gap2 hover:bg-panel")` → перевіряє кожен атом (і варіант-префікс перед `:`, якщо є)
//! проти граматики на етапі КОМПІЛЯЦІЇ, помилка з підказкою найближчого правильного імені
//! (Левенштейн) замість мовчазного рядка з друкарською помилкою, який ніколи не отримає стилю.
//!
//! Дані й комбінаторна логіка граматики — `include!("../../style/grammar_data.rs")`, ТОЙ САМИЙ
//! файл, що й `style::generate_css()` читає. Textual include, не crate-залежність — уникає циклу
//! `style` ⇄ `style_macros` (якби `style_macros` залежав від `style` для даних, а `style`
//! реекспортував `cx!` для ергономіки `style::cx!`, вийшов би цикл; `.claude/decisions/
//! style-system-architecture.md` розглядав цю альтернативу).

// Спільний grammar_data.rs несе більше даних/функцій, ніж style_macros конкретно споживає
// (style::generate_css потребує решти) — це навмисно, єдине джерело правди важливіше за
// мінімальний per-споживач набір (`.claude/decisions/style-system-architecture.md`).
#![allow(dead_code)]

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};

include!("../../style/grammar_data.rs");

fn closest(unknown: &str) -> Option<String> {
    all_atom_names()
        .into_iter()
        .min_by_key(|name| levenshtein(unknown, name))
}

fn closest_variant(unknown: &str) -> Option<&'static str> {
    VARIANTS
        .iter()
        .min_by_key(|v| levenshtein(unknown, v))
        .copied()
}

/// Розкладає один токен `cx!`-рядка на (варіант-префікси, базовий атом). `hover:focus-visible:
/// bg-panel` — кілька варіантів дозволено (складені стани), останній сегмент — завжди базовий
/// атом.
fn split_variants(token: &str) -> (Vec<&str>, &str) {
    let mut parts: Vec<&str> = token.split(':').collect();
    let base = parts.pop().unwrap_or(token);
    (parts, base)
}

/// Валідує рядок атомів (через пробіл), кожен — опційно з варіант-префіксами через `:`. Повертає
/// рядок як `&'static str` (сам рядок — водночас список CSS-класів: ім'я атома/варіанта ==
/// ім'я CSS-класу в `style::generate_css`, варіанти екрануються там же).
#[proc_macro]
pub fn cx(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let value = lit.value();

    let mut problems = Vec::new();
    for token in value.split_whitespace() {
        let (variants, base) = split_variants(token);
        for v in &variants {
            if !VARIANTS.contains(v) {
                let msg = match closest_variant(v) {
                    Some(s) => format!("невідомий варіант «{v}:» — може, ви мали на увазі «{s}:»?"),
                    None => format!("невідомий варіант «{v}:»"),
                };
                problems.push(msg);
            }
        }
        if !is_valid_atom(base) {
            let msg = match closest(base) {
                Some(s) => format!("невідомий атом «{base}» — може, ви мали на увазі «{s}»?"),
                None => format!("невідомий атом «{base}»"),
            };
            problems.push(msg);
        }
    }

    if !problems.is_empty() {
        let msg = problems.join("; ");
        return syn::Error::new(lit.span(), msg).to_compile_error().into();
    }

    quote! { #value }.into()
}
