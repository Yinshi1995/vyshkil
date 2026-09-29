# style/ — токени й граматика атомів як Rust-дані (Фаза 0, `docs/spec/08-style-system.md`)

- Без залежностей у `[dependencies]` (компілюється нативно й wasm32); `leptos`+`style_macros` —
  лише `[dev-dependencies]` для прототипу/тестів, не для рантайму бібліотеки.
- Джерело істини для CSS-генератора, `cx!`-валідації (дублюється в `style_macros`, задокументоване
  Фаза-0-обмеження — `.claude/decisions/style-system-architecture.md`) і майбутнього контраст-тесту.
- Значення атома — лише `var(--токен)` або bare-keyword без шкали (перевіряється тестом
  `every_atom_decl_references_a_token_var_or_is_a_bare_keyword`).
- Фаза 0 — мікро-прототип (11 токенів, 20 атомів). Повна граматика/теми/варіанти — Фаза 1.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `src/lib.rs` | `TOKENS`, `ATOMS`, `generate_css()`, `closest_atom()` | `style_macros` (дублює дані), `examples/preview.rs` |
| `tests/button.rs` | SSR-рендер кнопки з `cx!`, перевіряє клас у HTML і в `generate_css()` | — |
| `examples/preview.rs` | Дамп CSS+кнопки в `preview.html` для візуальної перевірки | ручний запуск |
