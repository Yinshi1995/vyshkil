---
date: 2026-10-02
status: accepted
---

# Рішення: Data Workspace — інтерфейс роботи з даними

## Контекст

Дані потрапляють у систему через імпорт файлів, але після цього їх неможливо
редагувати, вводити вручну, видаляти чи змінювати через UI. Потрібен потужний
інструмент роботи з даними, натхненний Power BI + Excel + Notion tables.

## Рішення

Замінити поточні read-only сторінки `/training` і `/training-form` єдиною
сторінкою `/data` з повнофункціональним DataGrid та CRUD API.

### Ключові можливості

1. **DataGrid з інлайн-редагуванням** (Excel-like):
   - Клік на клітинку → редагування на місці
   - Tab/Enter навігація між клітинками
   - Розумні типи клітинок: текст, число, select (combobox), дата
   - Віртуалізація для сотень рядків

2. **Фільтри** (Notion-like):
   - Швидкі фільтри: вид підготовки (чіпси), пошук
   - Розширені: колонка → оператор → значення
   - Групування за будь-якою колонкою

3. **Summary bar** (Power BI-like):
   - KPI-плитки: груп, план, прибуло, навчається
   - Оновлюються з фільтрами

4. **Detail panel** (бічна панель):
   - Воронка групи
   - Хронологія подій (event sourcing)
   - Додавання/редагування подій

5. **CRUD**:
   - Створення: додати рядок + заповнити inline
   - Редагування: inline в кожній клітинці
   - Видалення: через меню рядка, з підтвердженням
   - Збереження через audit_log, як і решта

### Стек

- **@tanstack/react-table v8** — headless table logic
- **@tanstack/react-virtual** — віртуалізація рядків
- Existing shadcn/ui components
- Server: нові API endpoints в `server/src/api.rs`

### API endpoints

```
GET    /api/data/groups          — список груп з фільтрами/сортуванням
POST   /api/data/groups          — створити групу
PUT    /api/data/groups/:id      — оновити поля групи
DELETE /api/data/groups/:id      — видалити групу (soft)
GET    /api/data/groups/:id/events — хронологія подій
POST   /api/data/groups/:id/events — додати подію
```

### Чого НЕ робимо в першій ітерації

- Views system (збережені конфігурації колонок/фільтрів) — потім
- Drag-and-drop стовпців — потім
- Copy/paste блоків клітинок — потім
- Pivot tables — потім

## Альтернативи

1. **Окрема бібліотека (AG Grid / Handsontable)** — відхилено: тягне 200KB+,
   несумісна зі shadcn-стилізацією, TanStack Table headless і легший.
2. **Leptos grid (існуючий)** — відхилено: міграція на React вже відбулася,
   Leptos grid не підтримується.
