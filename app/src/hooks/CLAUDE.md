# app/src/hooks — реактивні хелпери

- Можна: `types`, `domain`, `leptos`. Не можна: `sea_orm`/`backend` (хук — про реактивність
  клієнта, не про доступ до БД).
- Хук лише централізує звернення до контексту/сигналу — без побічної логіки (та йде в `domain`
  чи в сам компонент, залежно від того, чиста вона чи ні).

| Елемент | Що це | Хто використовує |
|---|---|---|
| `use_actor.rs` | `use_actor()` — поточний `RwSignal<Option<Actor>>` з контексту | `layout::ActorSwitcher`, `pages::home`, `pages::org_detail` |
| `use_escape_close.rs` | `use_escape_close(open, on_close)` — Escape закриває, поки `open` | `components::Modal`, `components::Drawer` |
| `use_floating_position.rs` | `use_floating_position(root, open)` — `(flip_up, align_end)`, евристика "де більше місця" відносно в'юпорту (feedback користувача — панелі вилазили за екран) | `components::Select`, `components::DatePicker`, `widgets::group_grid::autocomplete` |
| `use_popover_position.rs` | `use_popover_position(root, open)` — реальні пікселі (`position: fixed`) для ПОРТАЛЬОВАНОЇ панелі (`docs/spec/components/select.md` §4) — розширює ідею `use_floating_position` на портал, де CSS-класів in-flow вже не досить | `components::Combobox` |
