// Знеособлений шаблон (04-reconciliation-notifications.md §5): "Ні назв частин, номерів,
// кількостей, ВОС, місць." Один текст на template-варіант -- нотифікатор сам вирішує ЩО сказати,
// брокер ніколи не несе вільний текст (09 §3.5).

import type { NotifyTemplate } from "./generated/notify_send.v1.d.ts";

const TEMPLATES: Record<NotifyTemplate, string> = {
  discrepancy_detected:
    "Виявлено розбіжності у ваших даних щодо підготовки. Увійдіть у систему та перевірте розділ \"Розбіжності\".",
  discrepancy_resolved:
    "Розбіжності усунено. Дані узгоджені — перевірте розділ \"Розбіжності\".",
  import_completed:
    "Імпорт даних завершено. Увійдіть у систему та перевірте результати.",
  submission_committed:
    "Нове подання зафіксовано. Увійдіть у систему та перевірте дані.",
  group_event_added:
    "Додано подію групи підготовки. Увійдіть у систему та перевірте розклад.",
};

export function renderTemplate(template: NotifyTemplate): string {
  return TEMPLATES[template];
}
