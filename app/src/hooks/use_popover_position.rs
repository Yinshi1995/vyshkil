//! Позиціонування ПОРТАЛЬОВАНОЇ панелі (`components::Combobox`, `docs/spec/components/select.md`
//! §4) — на відміну від `use_floating_position` (CSS-класи для in-flow панелі поруч із тригером),
//! тут панель монтується в `document.body` через `leptos::portal::Portal`, тож позиціонування
//! рахує РЕАЛЬНІ пікселі (`position: fixed`) — Leptos `style:property=`-біндинги тут єдиний
//! правильний спосіб (динамічна per-instance геометрія, не візуальний рецепт — не порушує "атоми,
//! не інлайн-стилі": архітектурний тест ловить лише буквальний статичний style-атрибут, не цю
//! форму).

use leptos::prelude::*;

/// `min_width`/`max_height` — уже готові px-значення для `style:`-біндингів на панелі.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopoverPosition {
    pub top: Option<f64>,
    pub bottom: Option<f64>,
    pub left: Option<f64>,
    pub right: Option<f64>,
    pub min_width: f64,
    pub max_height: f64,
}

const GAP_PX: f64 = 4.0;
const MAX_PANEL_HEIGHT: f64 = 280.0;
const MIN_SPACE_PX: f64 = 240.0;

/// `None`, поки закрито (панель ще не змонтована/не потрібна). Рахується заново щоразу, коли
/// `open` стає `true` — досить для панелі, що закривається першим-ліпшим кліком-поза/Escape/
/// скролом сторінки (§4 "close on scroll" — сторінка сама викликає `close`, цей хук лише міряє).
pub fn use_popover_position(
    root: NodeRef<leptos::html::Div>,
    open: Signal<bool>,
) -> Signal<Option<PopoverPosition>> {
    let position = RwSignal::new(None::<PopoverPosition>);

    Effect::new(move |_| {
        if !open.get() {
            position.set(None);
            return;
        }
        if !cfg!(target_arch = "wasm32") {
            return;
        }
        let Some(el) = root.get_untracked() else { return };
        let rect = el.get_bounding_client_rect();
        let Some(window) = web_sys::window() else { return };
        let win_h = window.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
        let win_w = window.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);

        let space_below = win_h - rect.bottom();
        let space_above = rect.top();
        let flip_up = space_below < MIN_SPACE_PX && space_above > space_below;

        let space_right = win_w - rect.left();
        let align_end = space_right < MIN_SPACE_PX && rect.right() > space_right;

        let available = if flip_up { space_above } else { space_below };
        let max_height = (available - GAP_PX * 2.0).clamp(120.0, MAX_PANEL_HEIGHT);

        position.set(Some(PopoverPosition {
            top: (!flip_up).then_some(rect.bottom() + GAP_PX),
            bottom: flip_up.then_some(win_h - rect.top() + GAP_PX),
            left: (!align_end).then_some(rect.left()),
            right: align_end.then_some(win_w - rect.right()),
            min_width: rect.width(),
            max_height,
        }));
    });

    position.into()
}
