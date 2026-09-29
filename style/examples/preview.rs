//! Дамп прев'ю мікро-прототипу (Фаза 0) в один HTML-файл — CSS з `generate_css()` +
//! рендер кнопки — для візуальної перевірки скриншотом. Не частина рушія, лише перевірка "Фази 0".

use leptos::prelude::*;
use style_macros::cx;

#[component]
fn Button() -> impl IntoView {
    view! {
        <button class=cx!("flex items-c gap2 p2 bg-panel fg-accent r1")>
            "Зафіксувати"
        </button>
    }
}

fn main() {
    let css = style::generate_css();
    let button_html = Button().to_html();
    let html = format!(
        r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<style>
  body {{ background: #0d0f0a; padding: 40px; font-family: sans-serif; }}
{css}
</style>
</head>
<body>
{button_html}
</body>
</html>
"#
    );
    std::fs::write("preview.html", html).unwrap();
    println!("written: preview.html");
}
