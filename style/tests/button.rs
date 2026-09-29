//! Мікро-прототип "одна кнопка": перевіряє, що `style_macros::cx!` реально приймає Leptos
//! `class=`, і що згенерований `style::generate_css()` містить усі класи, які кнопка вживає.
//! Рендерить SSR у звичайний Rust-тест (нативний, не wasm) — жодного коду в `app/`.

use leptos::prelude::*;
use style_macros::cx;

#[component]
fn Button() -> impl IntoView {
    view! {
        <button class=cx!("flex items-c gap2 p2 bg-panel fg-accent r1 hover:bg-raised focus-visible:bd-accent")>
            "Зафіксувати"
        </button>
    }
}

#[test]
fn button_renders_with_validated_atom_classes() {
    let html = Button().to_html();

    assert!(html.contains("<button"), "має бути тег button: {html}");
    for atom in [
        "flex",
        "items-c",
        "gap2",
        "p2",
        "bg-panel",
        "fg-accent",
        "r1",
        "hover:bg-raised",
    ] {
        assert!(
            html.contains(atom),
            "клас «{atom}» відсутній у виводі: {html}"
        );
    }

    // Кожен клас, який кнопка реально вживає, має існувати в генерованому CSS (варіант — з
    // екранованим `:` і реальним псевдокласом `:hover`).
    let css = style::generate_css();
    for atom in [
        "flex",
        "items-c",
        "gap2",
        "p2",
        "bg-panel",
        "fg-accent",
        "r1",
    ] {
        assert!(
            css.contains(&format!(".{atom} {{")),
            "CSS для «{atom}» відсутній у generate_css()"
        );
    }
    assert!(
        css.contains(".hover\\:bg-raised:hover {"),
        "CSS для варіанта «hover:bg-raised» відсутній у generate_css()"
    );
    assert!(
        css.contains(".focus-visible\\:bd-accent:focus-visible {"),
        "CSS для варіанта «focus-visible:bd-accent» відсутній у generate_css()"
    );
}
