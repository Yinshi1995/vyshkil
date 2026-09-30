//! Позиціонування плаваючої панелі (`Select`/`DatePicker`/`OrgAutocomplete`-дропдаун) відносно
//! в'юпорту — евристика "де більше місця" (не повний floating-ui: міряє лише тригер, бо сама
//! панель ще не змонтована, поки `open=false` — вимірювати ЇЇ розмір довелось би вже ПІСЛЯ
//! рендеру, окремим кадром; тут досить грубого порогу, щоб перестати вилазити за екран).

use leptos::prelude::*;

/// Панелі цього застосунку — короткі списки/календар (`max-height` ~280px) — типовий поріг.
const THRESHOLD_PX: f64 = 240.0;

/// `(flip_up, align_end)` — застосувати як `class:...--flip-up=flip_up`/`class:...--align-end=
/// align_end` на панель. Рахується наново щоразу, коли `open` стає `true` (досить для панелі, що
/// закривається першим-ліпшим кліком-поза/Escape — ганяти resize-обробник для неї не варто).
pub fn use_floating_position(
    root: NodeRef<leptos::html::Div>,
    open: Signal<bool>,
) -> (Signal<bool>, Signal<bool>) {
    let flip_up = RwSignal::new(false);
    let align_end = RwSignal::new(false);

    Effect::new(move |_| {
        if !open.get() || !cfg!(target_arch = "wasm32") {
            return;
        }
        let Some(el) = root.get_untracked() else { return };
        let rect = el.get_bounding_client_rect();
        let Some(window) = web_sys::window() else { return };
        let win_h = window.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
        let win_w = window.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);

        let space_below = win_h - rect.bottom();
        let space_above = rect.top();
        flip_up.set(space_below < THRESHOLD_PX && space_above > space_below);

        let space_right = win_w - rect.left();
        let space_left = rect.right();
        align_end.set(space_right < THRESHOLD_PX && space_left > space_right);
    });

    (flip_up.into(), align_end.into())
}
