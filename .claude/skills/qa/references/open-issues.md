# Відкриті знахідки (оновлювати наприкінці кожного прогону)

Формат: серйозність · зона · що · `file:line` · статус (VERIFIED / SUSPECTED).

## Відомо станом на 2026-11-02

- LOW · UI · Консоль на `/data`: "Can't perform a React state update on a component that hasn't mounted yet" —
  побічний ефект у рендері, джерело не знайдено · `web/src/pages/data-workspace.tsx` · SUSPECTED
- LOW · типи · `tsc -b` падає на старих помилках (`web/src/api/mock.ts` — нема `callsign`, `avatar_url`,
  `can_see_org_names`; `web/src/lib/d4-pptx.ts` — типи `pptxgenjs`). Прод збирається `vite build` без `tsc`, тож
  не блокує · VERIFIED
- LOW · UI · `oxlint`: `useMemo` без залежності `searchParams` · `web/src/pages/dashboard.tsx` (initFilters) · VERIFIED
- MEDIUM · UI · Автокомпліти (підрозділ, ВОС, організатор, полігон, місто) у формі "Нова група" не перевірені
  з реальними даними: моки не віддають результатів пошуку. Користувач повідомляв, що автокомпліти
  "провалювалися" під іншими компонентами — відтворити на стенді з БД · NOT CHECKED
- LOW · деплой · Автодеплой після пушу `2f0699d` на прод не підтверджено (SSH-ключ на VyOS не підійшов) · NOT CHECKED

## Виправлено

- 2026-11-02 · `DropdownMenuTrigger` з `asChild` рендерив `button` у `button` (шапка, дашборд) → `render`-проп ·
  `web/src/components/ui/dropdown-menu.tsx`
- 2026-11-02 · Головна показувала сирий `in_training_count` → переклад "Навчається" · `web/src/pages/home.tsx`
