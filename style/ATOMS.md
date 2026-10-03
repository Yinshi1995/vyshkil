# ATOMS.md — шпаргалка атомів

АВТО-ЗГЕНЕРОВАНО з `grammar_data.rs` — НЕ РЕДАГУВАТИ ВРУЧНУ, `cargo run -p style --bin gen`.
Повний опис/приклади/do-don't — `docs/spec/08-style-system.md`.

Клас = `cx!("атом атом ...")` — компілятор валідує кожен, підказує typo.

## Відступи (шкала 0..8 = 0/4/8/12/16/24/32/48/64px)

`p` `px` `py` `pt` `pr` `pb` `pl` `m` `mx` `my` `mt` `mr` `mb` `ml` `gap` — кожен як `{префікс}0`..`{префікс}8`, напр. `p2` = padding 8px

## Шкали без варіацій властивості

- Радіус: `r0` `r1` `r2` `r3`
- Letter-spacing: `track0` `track1` `track2` `track3` `track4` `track5` `track6`
- Текст: `t-xs` `t-sm` `t-md` `t-lg` `t-xl` `t-display`
- Тінь: `shadow0` `shadow1` `shadow2`
- Z-індекс: `z-dropdown` `z-overlay` `z-modal` `z-toast`
- Тривалість: `duration-fast` `duration-base` `duration-slow`

## Ключові слова (без шкали)

`flex` `col` `row` `wrap` `grid` `cols-1` `cols-2`
`cols-3` `cols-4` `cols-6` `items-s` `items-c` `items-e` `justify-s`
`justify-c` `justify-e` `justify-b` `w-full` `w-auto` `maxw-prose` `maxw-full`
`fw4` `fw5` `fw7` `up` `num` `mono` `head`
`fg-main` `fg-muted` `fg-subtle` `fg-accent` `fg-ok` `fg-warn` `fg-danger`
`fg-info` `bg-base` `bg-panel` `bg-raised` `bg-accent` `fg-on-accent` `bg-accent-dim`
`bd` `bd-accent` `chamfer`
`bracket` (кутові скоби, тактичний мотив)

## Варіанти (префікс перед `:`)

`hover:` `focus-visible:` `active:` `disabled:` `invalid:` `open:` `md:` `lg:` `@md:`
Приклад: `hover:bg-raised`. `md:`/`lg:` — min-width медіа; `@md:` — container query.

## Теми

night · day — `<html data-theme="...">`, SSR виставляє атрибут (без блимання).

## Примітивні кольори (НЕ вживати напряму — лише через `fg-`/`bg-`/`bd-`)

`ink-N` `gold-N` `olive-N` `amber-N` `brick-N` `steel-N`
