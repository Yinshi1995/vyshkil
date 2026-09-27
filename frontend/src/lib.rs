use app::app::App;

// Єдина точка входу в WASM: браузер викликає її після завантаження .wasm,
// далі Leptos "оживляє" (hydrate) вже відрендерений сервером HTML замість повного клієнтського рендеру.
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
