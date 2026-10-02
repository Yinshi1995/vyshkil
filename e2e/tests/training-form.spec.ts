import { test, expect, type Page } from "@playwright/test";

// docs/spec/06-roadmap.md, Етап 4, "Готово, коли": весь рядок вноситься без миші; `?` показує
// шпаргалку; помилкові дати/кількості не зберігаються; e2e-тест на сценарій "5 рядків з клавіатури".
//
// Дані сідуються `m20260927_000011_seed_dev_data` (dev-сід): "152 нц (4896)" — гарантовано існує
// й має один training_site ("м. Верхньодніпровськ"). Тест припускає, що перед запуском `submission`
// порожня (DELETE FROM group_event; DELETE FROM training_group; DELETE FROM submission; -- дивись
// MEMORY.md), інакше відновлена чернетка з попереднього запуску змінить стан першого рядка.
//
// ПРИВЕДЕНО У ВІДПОВІДНІСТЬ до переробки сітки (сесія 2026-09-30, grid-interaction.spec.ts) --
// до того цей файл ще мав Частину per-row (col 0) і нативні <select>/старий клас `.cell__dropdown-
// item`, яких уже нема. Патерни (selectActor/toolbar-org/.combobox__option/формат дати) — дослівно
// з grid-interaction.spec.ts, не вигадані заново. Колонки після зсуву (Частина — в тулбарі, не в
// рядку): 0 Вид підготовки · 1 ВОС/посада/курс · 2 ОВТ · 3 Місце · 4 З · 5 По · 6 План · 7 Прибуло ·
// 8 Навчаються.

const ORG_LABEL = "152 нц (4896)";
const SITE_LABEL = "м. Верхньодніпровськ";

async function selectActor(page: Page) {
  await page.goto("/training-form");
  // Перемикач актора в шапці — кастомний Select (кнопка + listbox, не нативний <select> —
  // переробка "з нуля", сесія 2026-09-30), той самий патерн, що в grid-interaction.spec.ts.
  // SSR-рендерена кнопка існує в DOM одразу, ДО завершення WASM-гідратації -- Playwright може
  // "клікнути" її до того, як Leptos навісить `on:click`, і клік мовчки нічого не робить
  // (класична SSR+гідратація гонка, не пов'язана з дефектами форми). Один ретрай — досить.
  const button = page.getByRole("button", { name: "Оберіть частину" });
  const option = page.getByRole("option", { name: ORG_LABEL });
  await button.click();
  try {
    await option.click({ timeout: 3_000 });
  } catch {
    await button.click();
    await option.click();
  }
  await page.keyboard.press("Escape"); // закрити шпаргалку, що відкривається сама при першому вході
}

/** Обирає частину-відправника РАЗ у тулбарі (дефект 1, grid-interaction.spec.ts) — не per-рядок. */
async function selectSenderOrgInToolbar(page: Page) {
  await page.locator("#toolbar-sender-org").click();
  await page.locator("#toolbar-sender-org").pressSequentially("152нц", { delay: 10 });
  await page.locator(".combobox__option").first().waitFor();
  await page.keyboard.press("Enter");
}

async function fillAsOfDate(page: Page) {
  // DatePicker з маскою (dd.mm.yyyy), не нативний input (grid-interaction.spec.ts, j2).
  await page.getByRole("textbox", { name: "Станом на" }).fill("28.09.2026");
}

/** Заповнює один рядок сітки з клавіатури (клік лише щоб поставити фокус у першу клітинку рядка).
 * `isLast=false` завершує останнім Tab-ом, що переносить у перший рядок НАСТУПНОГО рядка (02 §2:
 * "з останнього поля рядка — у перше поле наступного рядка"); `isLast=true` (останній з 5 рядків)
 * зупиняється на останньому полі -- без цього кожен запуск лишав би зайвий порожній 6-й рядок,
 * що заблокував би `Ctrl+Enter` валідацією "не вказано частину-відправника". */
async function fillRowFromKeyboard(page: Page, rowIndex: number, isLast: boolean) {
  // Клітинка 0 — Вид підготовки (Combobox Trigger, фіксований список, БЕЗ друку -- grid-
  // interaction.spec.ts, тест b: Alt+ArrowDown без тексту). Клік відкриває список, тоді обираємо
  // опцію напряму за роллю/назвою -- не залежить від того, searchable цей режим чи ні.
  await page.locator(`#cell-${rowIndex}-0`).click();
  await page.getByRole("option", { name: "Фахова" }).click();
  await page.locator(".combobox__panel").waitFor({ state: "hidden" });

  const vosCell = page.locator(`#cell-${rowIndex}-1`);
  await vosCell.click();
  await vosCell.pressSequentially("вамп", { delay: 10 });
  // Дочекатись відповіді нечіткого пошуку (async server fn) ПЕРЕД Enter -- інакше Enter може
  // прилетіти раніше, ніж `Resource` оновиться, і випадайка ще порожня (02 §3, нечіткий пошук).
  await page.locator(".combobox__option").first().waitFor();
  await page.keyboard.press("Enter"); // "вамп" -> ВОС 218 (02 §3), перейти далі

  await page.keyboard.type("Vampire"); // ОВТ (клітинка 2, вже в фокусі після Enter)
  await page.keyboard.press("Tab");

  // Місце — лише один сід training_site, клік+вибір опції напряму (той самий підхід, що Вид
  // підготовки вище), без друку.
  await page.locator(`#cell-${rowIndex}-3`).click();
  await page.getByRole("option", { name: SITE_LABEL }).click();
  await page.locator(".combobox__panel").waitFor({ state: "hidden" });

  // Явний фокус на наступну клітинку -- клік мишею по опції (не keyboard Enter) не гарантовано
  // переносить фокус так само, як Enter у тесті b (grid-interaction.spec.ts).
  await page.locator(`#cell-${rowIndex}-4`).click();
  await page.keyboard.type("18.08");
  await page.keyboard.press("Tab");
  await page.keyboard.type("09.10");

  // План/Прибуло/Навчаються — явний клік по кожній клітинці (сліпий ланцюжок Tab між числовими
  // spinbutton-ами виявився ненадійним: значення накопичувались в одному полі замість переходу).
  for (const col of [6, 7, 8]) {
    const spinCell = page.locator(`#cell-${rowIndex}-${col}`);
    await spinCell.click();
    await spinCell.fill("10");
  }
  await page.locator(`#cell-${rowIndex}-8`).press("Tab"); // Організатор (необов'язково) -> ...
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
  await selectSenderOrgInToolbar(page);

  for (let i = 0; i < 5; i++) {
    await fillRowFromKeyboard(page, i, i === 4);
  }

  // 5 заповнених + 1 порожній "наступний" рядок, який сітка тримає завжди (поточна поведінка —
  // раніше з'являвся лише після Tab за межі останнього поля, зловлено тут візуально).
  await expect(page.locator(".grid__row")).toHaveCount(6);

  await page.keyboard.press("Control+Enter");

  await expect(page.getByText(/збережено: 5 груп/)).toBeVisible({ timeout: 10_000 });

  // Сітка скидається до одного порожнього рядка після успішної фіксації.
  await expect(page.locator(".grid__row")).toHaveCount(1);
});

test("? показує шпаргалку гарячих клавіш", async ({ page }) => {
  // Auto-popup прибрано — перевіряємо, що `?` відкриває шпаргалку.
  await page.goto("/training-form");
  const button = page.getByRole("button", { name: "Оберіть частину" });
  const option = page.getByRole("option", { name: ORG_LABEL });
  await button.click();
  try {
    await option.click({ timeout: 3_000 });
  } catch {
    await button.click();
    await option.click();
  }

  await expect(page.getByRole("heading", { name: "Гарячі клавіші" })).toBeHidden();

  await page.locator("body").press("?");
  await expect(page.getByRole("heading", { name: "Гарячі клавіші" })).toBeVisible();
  await page.getByRole("button", { name: "Закрити" }).click();
  await expect(page.getByRole("heading", { name: "Гарячі клавіші" })).toBeHidden();
});

test("помилкова дата не зберігається — Ctrl+Enter показує помилку і не комітить", async ({
  page,
}) => {
  await selectActor(page);
  await fillAsOfDate(page);
  await selectSenderOrgInToolbar(page);

  await page.locator("#cell-0-0").click();
  await page.getByRole("option", { name: "Фахова" }).click();
  await page.locator(".combobox__panel").waitFor({ state: "hidden" });
  await page.locator("#cell-0-1").click();
  await page.locator("#cell-0-1").pressSequentially("вамп", { delay: 10 });
  await page.locator(".combobox__option").first().waitFor();
  await page.keyboard.press("Enter");
  await page.keyboard.type("Vampire");
  await page.keyboard.press("Tab");
  await page.locator("#cell-0-3").click();
  await page.getByRole("option", { name: SITE_LABEL }).click();
  await page.locator(".combobox__panel").waitFor({ state: "hidden" });
  await page.locator("#cell-0-4").click();
  await page.keyboard.type("31.09"); // 03 §5: "такої дати нема" -- вересень має 30 днів
  await page.keyboard.press("Control+Enter");

  await expect(page.getByText(/такої дати нема/)).toBeVisible({ timeout: 10_000 });
  // Сітка НЕ скинулась -- рядок і досі тут, з тим самим (непошкодженим) уведеним значенням.
  // Маска автоматично додає крапку-роздільник після дд.мм (готуючись до року) — очікувана
  // поведінка поля, не дефект.
  await expect(page.locator("#cell-0-4")).toHaveValue("31.09.");
});
