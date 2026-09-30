//! Колонки Grid — опис/ширини/видимість (`docs/spec/components/grid.md` §1, §6, розділ Б).
//! Персистентність — `localStorage`, per-viewer зручність, без серверної сторони (той самий
//! `#[cfg(target_arch="wasm32")]`-гейт, що вже `set_theme` у `styleguide.rs`). Ручне
//! кодування рядком (не `serde_json`): той доступний лише під фічею `ssr`
//! (`app/Cargo.toml`), а цей модуль компілюється і для WASM.

use leptos::prelude::*;

/// Дія — фіксована 40px, не в цьому списку (не змінюється в ширину, завжди перша, sticky
/// `left: 0`). "Частина" — перший елемент тут, sticky `left: {ACTION_WIDTH}px` (константа, не
/// рахується в рантаймі — дія не resizable, зсув завжди той самий).
pub const ACTION_WIDTH_PX: f64 = 40.0;

pub struct ColumnDef {
    pub label: &'static str,
    pub min_px: f64,
    pub default_px: f64,
    /// "Частина" — sticky, завжди видима, не в тулбарі "показати колонки" (§6 спеки).
    pub toggleable: bool,
}

pub const DATA_COLUMNS: &[ColumnDef] = &[
    ColumnDef { label: "Частина", min_px: 160.0, default_px: 200.0, toggleable: false },
    ColumnDef { label: "Вид підготовки", min_px: 130.0, default_px: 160.0, toggleable: true },
    ColumnDef { label: "ВОС / посада / курс", min_px: 180.0, default_px: 220.0, toggleable: true },
    ColumnDef { label: "ОВТ", min_px: 130.0, default_px: 160.0, toggleable: true },
    ColumnDef { label: "Місце", min_px: 160.0, default_px: 200.0, toggleable: true },
    ColumnDef { label: "З", min_px: 90.0, default_px: 110.0, toggleable: true },
    ColumnDef { label: "По", min_px: 90.0, default_px: 110.0, toggleable: true },
    ColumnDef { label: "План", min_px: 64.0, default_px: 80.0, toggleable: true },
    ColumnDef { label: "Прибуло", min_px: 64.0, default_px: 80.0, toggleable: true },
    ColumnDef { label: "Навчаються", min_px: 64.0, default_px: 80.0, toggleable: true },
];

const WIDTHS_KEY: &str = "taktoblik.grid.widths.v1";
const VISIBLE_KEY: &str = "taktoblik.grid.visible.v1";

#[derive(Clone, Copy)]
pub struct ColumnsState {
    /// Той самий порядок/індекс, що `DATA_COLUMNS`.
    pub widths: RwSignal<Vec<f64>>,
    pub visible: RwSignal<Vec<bool>>,
}

impl ColumnsState {
    pub fn width(&self, i: usize) -> f64 {
        self.widths.get().get(i).copied().unwrap_or(DATA_COLUMNS[i].default_px)
    }

    pub fn set_width(&self, i: usize, px: f64) {
        let min = DATA_COLUMNS[i].min_px;
        self.widths.update(|w| {
            if let Some(slot) = w.get_mut(i) {
                *slot = px.max(min);
            }
        });
    }

    pub fn reset_width(&self, i: usize) {
        self.set_width(i, DATA_COLUMNS[i].default_px);
    }

    pub fn is_visible(&self, i: usize) -> bool {
        self.visible.get().get(i).copied().unwrap_or(true)
    }

    pub fn toggle_visible(&self, i: usize) {
        if !DATA_COLUMNS[i].toggleable {
            return;
        }
        self.visible.update(|v| {
            if let Some(slot) = v.get_mut(i) {
                *slot = !*slot;
            }
        });
    }

    /// `style:grid-template-columns` — дія (фіксована) + видимі колонки даних. Легітимний
    /// `style:`-виняток (динамічна геометрія, не візуальний рецепт) — той самий, що вже
    /// в `components::Combobox`.
    pub fn grid_template_columns(&self) -> String {
        let mut parts = vec![format!("{ACTION_WIDTH_PX}px")];
        for (i, _) in DATA_COLUMNS.iter().enumerate() {
            if self.is_visible(i) {
                parts.push(format!("{}px", self.width(i)));
            }
        }
        parts.join(" ")
    }
}

fn parse_csv_f64(raw: &str) -> Vec<f64> {
    raw.split(',').filter_map(|s| s.trim().parse().ok()).collect()
}

fn parse_csv_bool(raw: &str) -> Vec<bool> {
    raw.split(',').map(|s| s.trim() == "1").collect()
}

pub fn use_columns() -> ColumnsState {
    let widths = RwSignal::new(DATA_COLUMNS.iter().map(|c| c.default_px).collect::<Vec<_>>());
    let visible = RwSignal::new(vec![true; DATA_COLUMNS.len()]);

    Effect::new(move |_| {
        if !cfg!(target_arch = "wasm32") {
            return;
        }
        let Some(win) = web_sys::window() else { return };
        let Ok(Some(storage)) = win.local_storage() else { return };
        if let Ok(Some(raw)) = storage.get_item(WIDTHS_KEY) {
            let parsed = parse_csv_f64(&raw);
            if parsed.len() == DATA_COLUMNS.len() {
                widths.set(parsed);
            }
        }
        if let Ok(Some(raw)) = storage.get_item(VISIBLE_KEY) {
            let parsed = parse_csv_bool(&raw);
            if parsed.len() == DATA_COLUMNS.len() {
                visible.set(parsed);
            }
        }
    });

    Effect::new(move |_| {
        let w = widths.get();
        if !cfg!(target_arch = "wasm32") {
            return;
        }
        let Some(win) = web_sys::window() else { return };
        let Ok(Some(storage)) = win.local_storage() else { return };
        let raw = w.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",");
        let _ = storage.set_item(WIDTHS_KEY, &raw);
    });

    Effect::new(move |_| {
        let v = visible.get();
        if !cfg!(target_arch = "wasm32") {
            return;
        }
        let Some(win) = web_sys::window() else { return };
        let Ok(Some(storage)) = win.local_storage() else { return };
        let raw = v.iter().map(|b| if *b { "1" } else { "0" }).collect::<Vec<_>>().join(",");
        let _ = storage.set_item(VISIBLE_KEY, &raw);
    });

    ColumnsState { widths, visible }
}

/// Ручка перетягування правого краю колонки — `on:mousedown` на ній, дальші `mousemove`/`mouseup`
/// вішаються на `window` лише на час перетягування (знімаються самі при відпусканні).
pub fn start_resize(state: ColumnsState, col_index: usize, start_event: &web_sys::MouseEvent) {
    use leptos::ev;
    use leptos::leptos_dom::helpers::window_event_listener;
    use std::cell::RefCell;
    use std::rc::Rc;

    let start_x = start_event.client_x() as f64;
    let start_width = state.width(col_index);

    let move_handle: Rc<RefCell<Option<leptos::leptos_dom::helpers::WindowListenerHandle>>> =
        Rc::new(RefCell::new(None));
    let up_handle: Rc<RefCell<Option<leptos::leptos_dom::helpers::WindowListenerHandle>>> =
        Rc::new(RefCell::new(None));

    let move_handle_for_move = move_handle.clone();
    let up_handle_for_move = up_handle.clone();
    let mh = window_event_listener(ev::mousemove, move |ev: web_sys::MouseEvent| {
        let delta = ev.client_x() as f64 - start_x;
        state.set_width(col_index, start_width + delta);
    });

    let move_handle_for_up = move_handle.clone();
    let up_handle_for_up = up_handle.clone();
    let uh = window_event_listener(ev::mouseup, move |_| {
        if let Some(h) = move_handle_for_up.borrow_mut().take() {
            h.remove();
        }
        if let Some(h) = up_handle_for_up.borrow_mut().take() {
            h.remove();
        }
    });

    *move_handle_for_move.borrow_mut() = Some(mh);
    *up_handle_for_move.borrow_mut() = Some(uh);
}
