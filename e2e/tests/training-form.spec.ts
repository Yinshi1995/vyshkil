import { test, expect, type Page } from "@playwright/test";

// docs/spec/06-roadmap.md, Етап 4, "Готово, коли": весь рядок вноситься без миші; `?` показує
// шпаргалку; помилкові дати/кількості не зберігаються; e2e-тест на сценарій "5 рядків з клавіатури".
//
// Дані сідуються `m20260927_000011_seed_dev_data` (dev-сід): "152 нц (4896)" — гарантовано існує
// й має один training_site ("м. Верхньодніпровськ"). Тест припускає, що перед запуском `submission`
// порожня (DELETE FROM group_event; DELETE FROM training_group; DELETE FROM submission; -- дивись
// MEMORY.md), інакше відновлена чернетка з попереднього запуску змінить стан першого рядка.

const ORG_LABEL = "152 нц (4896)";
const SITE_LABEL = "м. Верхньодніпровськ";

async function selectActor(page: Page) {
  await page.goto("/training-form");
  // Перемикач актора в шапці — той самий <select>, що й на інших сторінках (не частина сітки,
  // клік тут не порушує "без миші" для самої форми). `<label>` не пов'язаний з `<select>` явно
  // (немає `for`/`id`), тож шукаємо через контейнер, а не `getByLabel`.
  await page.locator(".actor-switcher select").first().selectOption(ORG_LABEL);
  await page.keyboard.press("Escape"); // закрити шпаргалку, що відкривається сама при першому вході
}

async function fillAsOfDate(page: Page) {
  await page.getByRole("textbox", { name: "Станом на" }).fill("2026-09-28");
}

/** Заповнює один рядок сітки з клавіатури (клік лише щоб поставити фокус у першу клітинку рядка).
 * `isLast=false` завершує останнім Tab-ом, що переносить у перший рядок НАСТУПНОГО рядка (02 §2:
 * "з останнього поля рядка — у перше поле наступного рядка"); `isLast=true` (останній з 5 рядків)
 * зупиняється на останньому полі -- без цього кожен запуск лишав би зайвий порожній 6-й рядок,
 * що заблокував би `Ctrl+Enter` валідацією "не вказано частину-відправника". */
async function fillRowFromKeyboard(page: Page, rowIndex: number, isLast: boolean) {
  const orgCell = page.locator(`#cell-${rowIndex}-0`);
  await orgCell.click();
  await orgCell.pressSequentially("152нц", { delay: 10 });
  // Дочекатись відповіді нечіткого пошуку (async server fn) ПЕРЕД Enter -- інакше Enter може
  // прилетіти раніше, ніж `Resource` оновиться, і випадайка ще порожня (02 §3, нечіткий пошук).
  await page.locator(".cell__dropdown-item").first().waitFor();
  await page.keyboard.press("Enter"); // підтвердити топ-збіг автокомпліту, перейти далі (02 §2)

  await page.locator(`#cell-${rowIndex}-1`).selectOption("Фахова");
  await page.keyboard.press("Tab");

  await page.locator(`#cell-${rowIndex}-2`).pressSequentially("вамп", { delay: 10 });
  await page.locator(".cell__dropdown-item").first().waitFor();
  await page.keyboard.press("Enter"); // "вамп" -> ВОС 218 (02 §3), перейти далі

  await page.keyboard.type("Vampire"); // ОВТ (клітинка 3, вже в фокусі після Enter)
  await page.keyboard.press("Tab");

  await page.locator(`#cell-${rowIndex}-4`).selectOption(SITE_LABEL);
  await page.keyboard.press("Tab");

  await page.keyboard.type("18.08");
  await page.keyboard.press("Tab");
  await page.keyboard.type("09.10");
  await page.keyboard.press("Tab");
  await page.keyboard.type("10");
  await page.keyboard.press("Tab");
  await page.keyboard.type("10");
  await page.keyboard.press("Tab");
  await page.keyboard.type("10");
  await page.keyboard.press("Tab"); // Організатор (необов'язково) -> ...
  await page.keyboard.press("Tab"); // № розпорядження (необов'язково) -> ...
  await page.keyboard.press("Tab"); // Дата розпорядження (необов'язково) -> ...
  await page.keyboard.press("Tab"); // Примітка (необов'язково) -> останнє поле рядка
  if (!isLast) {
    await page.keyboard.press("Tab"); // з останнього поля -> перший рядок НАСТУПНОГО рядка
  }
}

test("5 рядків з клавіатури — цілий рядок без миші, фіксація без помилок", async ({ page }) => {
  await selectActor(page);
  await fillAsOfDate(page);

  for (let i = 0; i < 5; i++) {
    await fillRowFromKeyboard(page, i, i === 4);
  }

  await expect(page.locator(".grid__row")).toHaveCount(5);

  await page.keyboard.press("Control+Enter");

  await expect(page.getByText(/збережено: 5 груп/)).toBeVisible({ timeout: 10_000 });

  // Сітка скидається до одного порожнього рядка після успішної фіксації.
  await expect(page.locator(".grid__row")).toHaveCount(1);
});

test("? показує шпаргалку гарячих клавіш", async ({ page }) => {
  await page.goto("/training-form");
  await page.locator(".actor-switcher select").first().selectOption(ORG_LABEL);

  // Шпаргалка відкрита сама при першому вході (02 §2: "показується автоматично при першому
  // відкритті") -- перевіряємо це, тоді закриваємо і перевіряємо, що `?` відкриває знову.
  await expect(page.getByRole("heading", { name: "Гарячі клавіші" })).toBeVisible();
  await page.getByRole("button", { name: "Закрити" }).click();
  await expect(page.getByRole("heading", { name: "Гарячі клавіші" })).toBeHidden();

  await page.locator("body").press("?");
  await expect(page.getByRole("heading", { name: "Гарячі клавіші" })).toBeVisible();
});

test("помилкова дата не зберігається — Ctrl+Enter показує помилку і не комітить", async ({
  page,
}) => {
  await selectActor(page);
  await fillAsOfDate(page);

  const orgCell = page.locator("#cell-0-0");
  await orgCell.click();
  await orgCell.pressSequentially("152нц", { delay: 10 });
  await page.locator(".cell__dropdown-item").first().waitFor();
  await page.keyboard.press("Enter");
  await page.locator("#cell-0-1").selectOption("Фахова");
  await page.keyboard.press("Tab");
  await page.locator("#cell-0-2").pressSequentially("вамп", { delay: 10 });
  await page.locator(".cell__dropdown-item").first().waitFor();
  await page.keyboard.press("Enter");
  await page.keyboard.type("Vampire");
  await page.keyboard.press("Tab");
  await page.locator("#cell-0-4").selectOption(SITE_LABEL);
  await page.keyboard.press("Tab");
  await page.keyboard.type("31.09"); // 03 §5: "такої дати нема" -- вересень має 30 днів
  await page.keyboard.press("Control+Enter");

  await expect(page.getByText(/такої дати нема/)).toBeVisible({ timeout: 10_000 });
  // Сітка НЕ скинулась -- рядок і досі тут, з тим самим (непошкодженим) уведеним значенням.
  await expect(page.locator("#cell-0-5")).toHaveValue("31.09");
});
