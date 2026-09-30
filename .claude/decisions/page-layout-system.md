---
tags: [decision, layout, page-header, toolbar, content-width]
date: 2026-09-30
---

# Лейаут сторінок — PageHeader/Toolbar/ContentWidth (Етап 7.5)

**Контекст**: `docs/spec/components/layout.md` — повна анатомія. Тут — рішення й "чому".

**`<main>` (app.rs) втрачає власний `max-width`** — це БУВ корінь "контент обмежений вузьким
max-width, хоча таблиці потрібна вся ширина" (Section Б завершила sticky/scroll-верстку Grid, але
не могла показати повний ефект, бо `main{max-width:1200px}` різав ширину ще до Grid'а). Кожна
сторінка тепер сама оголошує ширину через `PageContent width=ContentWidth::{Data,Detail,Reading}`
— `Data` без обмеження, `Detail`/`Reading` тримають межу (1200px/720px) на своєму рівні.

**Пілот на 2 сторінках** (`/training-form`, `/org/:id`) — та сама ЗУПИНКА-дисципліна, що розділ
А/Б: показати шаблон на малому зрізі, дочекатись "ок". Користувач підтвердив — розкатано на
решту (`/import`→`Data`, `/`→`Detail`, `/dictionaries`→`Detail`, `/vos-lookup`→`Reading`), по
одному коміту на сторінку (`docs/spec/components/layout.md` §6 — таблиця з обґрунтуванням типу
ширини кожної). Усі 6 сторінок тепер на спільних `PageHeader`/`PageContent`.

**`--control-height` — новий токен**, не рецепт-специфічний костиль: різні контроли в `Toolbar`
(кнопка/Combobox-field/DatePicker-field) мають РІЗНІ шрифти/border-width, тож самого padding
недостатньо для однакової фактичної висоти — спільний `min-height` на кожному рецепті вирішує
"кнопки різного розміру" системно, не точковим підбором px під кожен випадок.

**`.btn--ghost` — новий рецепт**, третій поруч із primary/outline: "одна primary-дія на сторінці,
решта secondary/ghost" з брифу буквально потребує третього рівня (outline вже зайнятий під
secondary — Шпаргалка/Колонки стають ghost, не outline, щоб візуально не конкурувати з реальними
secondary-діями).

Див. також [[grid-column-strategy]], [[combobox-rebuild-section-a]].
