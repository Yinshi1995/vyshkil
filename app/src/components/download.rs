//! Ініціює браузерне скачування байтів без справжнього URL-ендпоінта (Етап 7, `pages::documents`):
//! `#[server]`-функція повертає готові байти файлу напряму (як і решта server fn цього проєкту),
//! клієнт загортає їх у `Blob` + `<a download>`, клікає програмно, звільняє object URL.

use wasm_bindgen::JsCast;

pub fn download_bytes(bytes: &[u8], filename: &str, mime: &str) {
    if !cfg!(target_arch = "wasm32") {
        return;
    }

    let array = js_sys::Uint8Array::from(bytes);
    let parts = js_sys::Array::new();
    parts.push(&array.buffer());
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type(mime);
    let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &opts) else {
        return;
    };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else { return };

    let Some(window) = web_sys::window() else { return };
    let Some(document) = window.document() else { return };
    let Ok(el) = document.create_element("a") else { return };
    let Ok(anchor) = el.dyn_into::<web_sys::HtmlAnchorElement>() else { return };
    anchor.set_href(&url);
    anchor.set_download(filename);
    anchor.click();

    let _ = web_sys::Url::revoke_object_url(&url);
}
