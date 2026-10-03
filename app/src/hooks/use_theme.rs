use leptos::prelude::*;

/// Глобальний сигнал поточної теми — надається у `App` через `provide_context`.
pub type ThemeSignal = RwSignal<&'static str>;

/// Порядок тем для перемикача: night → day → night.
const THEME_ORDER: &[&str] = &["night", "day"];

/// Повертає наступну тему після поточної.
pub fn next_theme(current: &str) -> &'static str {
    let idx = THEME_ORDER.iter().position(|&t| t == current).unwrap_or(0);
    THEME_ORDER[(idx + 1) % THEME_ORDER.len()]
}

/// Встановлює атрибут `data-theme` на `<html>`. Працює лише у WASM.
pub fn set_theme(name: &str) {
    if !cfg!(target_arch = "wasm32") {
        return;
    }
    if let Some(el) = document().document_element() {
        let _ = el.set_attribute("data-theme", name);
    }
}

/// Читає `ThemeSignal` із контексту.
pub fn use_theme() -> ThemeSignal {
    expect_context::<ThemeSignal>()
}
