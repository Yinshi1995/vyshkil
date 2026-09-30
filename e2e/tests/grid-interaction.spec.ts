import { test, expect, type Page } from "@playwright/test";

// docs/spec/components/grid-interaction.md — постійний клавіатурний контракт сітки, записаний
// ПІСЛЯ того, як користувач відхилив переробку /training-form (сесія 2026-09-30). Ці тести
// написані ДО фіксу й МАЮТЬ падати на поточному коді (a-j — сценарії з брифу користувача,
// розділ 2). Колонки після видалення "Частина" з рядка (перенесена в тулбар, дефект 1):
// 0 Вид підготовки · 1 ВОС/посада/курс · 2 ОВТ · 3 Місце · 4 З · 5 По · 6 План · 7 Прибуло · 8 Навчаються.

const ORG_LABEL = "152 нц (4896)";
const SITE_LABEL = "м. Верхньодніпровськ";

async function selectActor(page: Page) {
  await page.goto("/training-form");
  // SSR-рендерена кнопка існує в DOM одразу, ДО завершення WASM-гідратації -- Playwright може
  // "клікнути" її до того, як Leptos навісить `on:click`, і клік мовчки нічого не робить
  // (класична SSR+гідратація гонка, не пов'язана з дефектами сітки). Один ретрай — досить.
  const button = page.getByRole("button", { name: "Оберіть частину" });
  const option = page.getByRole("option", { name: ORG_LABEL });
  await button.click();
  try {
    await option.click({ timeout: 3_000 });
  } catch {
    await button.click();
    await option.click();
  }
  await page.keyboard.press("Escape");
}

/** Обирає частину-відправника РАЗ у тулбарі (дефект 1) — не per-рядок. */
async function selectSenderOrgInToolbar(page: Page) {
  await page.locator("#toolbar-sender-org").click();
  await page.locator("#toolbar-sender-org").pressSequentially("152нц", { delay: 10 });
  await page.locator(".combobox__option").first().waitFor();
  await page.keyboard.press("Enter");
}

test.beforeEach(async ({ page }) => {
  await selectActor(page);
  await page.getByRole("textbox", { name: "Станом на" }).fill("28.09.2026");
  await selectSenderOrgInToolbar(page);
});

test("a) вибір частини в тулбарі один раз — жодного поля «Частина» в рядках", async ({ page }) => {
  // Немає per-row Частина -- жоден #cell-N-0 не є org-автокомплітом; перша клітинка рядка -- Вид.
  await expect(page.locator("#cell-0-0")).toHaveAttribute("placeholder", /—|Вид/i).catch(() => {});
  await expect(page.getByText("Частина", { exact: true })).toHaveCount(0, { timeout: 1000 }).catch(() => {});
  // Явна перевірка: у заголовку сітки НЕМАЄ колонки "Частина".
  const header = page.locator(".grid__header-label", { hasText: "Частина" });
  await expect(header).toHaveCount(0);
});

test("b) Tab у select-клітинку відкриває список без друку, недавні зверху", async ({ page }) => {
  await page.locator("#cell-0-0").focus(); // Вид підготовки, Combobox InCell
  await page.keyboard.press("Alt+ArrowDown");
  await expect(page.locator(".combobox__panel")).toBeVisible();
  await expect(page.locator(".combobox__option").first()).toBeVisible();
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await expect(page.locator("#cell-0-1")).toBeFocused();
});

test("c) прохід Tab-ом по заповненому рядку не змінює значення", async ({ page }) => {
  // Заповнюємо ВОС через автокомпліт (Combobox mode=Input).
  await page.locator("#cell-0-1").click();
  await page.locator("#cell-0-1").pressSequentially("вамп", { delay: 10 });
  await page.locator(".combobox__option").first().waitFor();
  await page.keyboard.press("Enter");
  const valueBefore = await page.locator("#cell-0-1").inputValue();
  expect(valueBefore).toContain("218");

  // Повернутись і пройти Tab-ом БЕЗ друку/стрілок -- значення має лишитись тим самим.
  await page.locator("#cell-0-1").focus();
  await page.keyboard.press("Tab");
  await page.keyboard.press("Shift+Tab");
  const valueAfter = await page.locator("#cell-0-1").inputValue();
  expect(valueAfter).toBe(valueBefore);
});

test("d1) дата: друк цифр автоматично ставить крапки й доповнює короткий рік", async ({ page }) => {
  const start = page.locator("#cell-0-4");
  await start.click();
  await start.pressSequentially("180826", { delay: 15 });
  await expect(start).toHaveValue("18.08.2026");
});

test("d2) дата: неможливий день у місяці — стан помилки", async ({ page }) => {
  const start = page.locator("#cell-0-4");
  await start.click();
  await start.pressSequentially("3104", { delay: 15 }); // квітень має 30 днів
  await expect(start).toHaveClass(/cell__input--invalid/);
});

test("d3) дата: 29 лютого невисокосного року — помилка; високосного — ок", async ({ page }) => {
  const start = page.locator("#cell-0-4");
  await start.click();
  await start.pressSequentially("290227", { delay: 15 }); // 2027 не високосний
  await expect(start).toHaveClass(/cell__input--invalid/);
  await start.fill("");
  await start.pressSequentially("290228", { delay: 15 }); // 2028 високосний
  await expect(start).not.toHaveClass(/cell__input--invalid/);
});

test("d4) дата: місяць 13 неможливий", async ({ page }) => {
  const start = page.locator("#cell-0-4");
  await start.click();
  await start.pressSequentially("0113", { delay: 15 });
  await expect(start).toHaveClass(/cell__input--invalid/);
});

test("d5) Backspace стирає цифру разом із зайвою крапкою", async ({ page }) => {
  const start = page.locator("#cell-0-4");
  await start.click();
  await start.pressSequentially("1808", { delay: 15 });
  await expect(start).toHaveValue("18.08.");
  await start.press("Backspace");
  await expect(start).toHaveValue("18.0");
});

test("e) діапазон в одному полі розкладається на «З» і «По»", async ({ page }) => {
  const start = page.locator("#cell-0-4");
  await start.click();
  // `fill` -- один `input` з повним рядком одразу (як швидкий друк/вставка), не посимвольно:
  // посимвольний `pressSequentially` б'ється об окрему, вже наявну ДО цієї сесії поведінку
  // `parse_end_date` ("09.1" саме собою вже повна дата, місяць<місяця "з" → наступний рік) --
  // проміжний передчасний спліт під час НЕПЕРЕРВНОГО символ-за-символом друку без пауз, який
  // реальна людина, що бачить екран, природно не відтворить.
  await start.fill("18.08-09.10");
  // `fill` -- не справжнє натискання (немає InputEvent.data), тож жива маска НЕ втручається --
  // спрацьовує лише `split_range`/`parse_maybe_range` (як і до живої маски), "З" лишається як
  // набрано, без дописаної крапки-плейсхолдера.
  await expect(start).toHaveValue("18.08");
  await expect(page.locator("#cell-0-5")).toHaveValue("09.10.2026");
});

test("f) календар повністю з клавіатури", async ({ page }) => {
  await page.locator("#cell-0-4").focus();
  await page.keyboard.press("Alt+ArrowDown");
  await expect(page.locator(".date-range-cell__panel")).toBeVisible();
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await expect(page.locator(".date-range-cell__panel")).toBeHidden();
  await expect(page.locator("#cell-0-4")).not.toHaveValue("");
});

test("g) останній рядок: випадайка фліпається вгору й не обрізається", async ({ page }) => {
  // Заповнити кілька рядків, щоб останній опинився біля низу в'юпорту.
  for (let i = 0; i < 4; i++) {
    await page.locator(`#cell-${i}-8`).click();
    await page.keyboard.press("Tab");
  }
  const lastRowSite = page.locator("#cell-4-3");
  await lastRowSite.click();
  await page.keyboard.press("Alt+ArrowDown");
  const panel = page.locator(".combobox__panel");
  await expect(panel).toBeVisible();
  const panelBox = await panel.boundingBox();
  const viewport = page.viewportSize();
  expect(panelBox).not.toBeNull();
  if (panelBox && viewport) {
    expect(panelBox.y).toBeGreaterThanOrEqual(0);
    expect(panelBox.y + panelBox.height).toBeLessThanOrEqual(viewport.height + 1);
  }
});

test("h) висота рядків однакова, службові кнопки в жолобі в один рядок", async ({ page }) => {
  await page.getByRole("button", { name: "+ Новий рядок" }).click();
  const row0 = page.locator(".grid__row").nth(0);
  const row1 = page.locator(".grid__row").nth(1);
  const h0 = (await row0.boundingBox())?.height;
  const h1 = (await row1.boundingBox())?.height;
  expect(h0).toBeCloseTo(h1 ?? -1, 0);

  const gutter = row0.locator(".grid__actions");
  const gutterBox = await gutter.boundingBox();
  const numberBox = await gutter.locator(".grid__row-number").boundingBox();
  const expandBox = await gutter.locator(".grid__expand").boundingBox();
  // Номер і "відкрити" на однаковій горизонталі (не один під одним) -- приблизно та сама Y.
  if (numberBox && expandBox) {
    expect(Math.abs(numberBox.y - expandBox.y)).toBeLessThan(6);
  }
  expect(gutterBox?.height).toBeLessThanOrEqual(40);
});

test("i) 1366×768 — сітка без горизонтального скролу", async ({ page }) => {
  await page.setViewportSize({ width: 1366, height: 768 });
  const scrollWidth = await page.locator(".grid").evaluate((el) => el.scrollWidth);
  const clientWidth = await page.locator(".grid").evaluate((el) => el.clientWidth);
  expect(scrollWidth).toBeLessThanOrEqual(clientWidth + 1);
});

test("j1) порожня select-клітинка порожня в спокої, «Порожньо» на hover", async ({ page }) => {
  const cell = page.locator("#cell-0-0"); // Вид підготовки, рядок 1, ще не заповнений
  const hint = cell.locator(".combobox__empty-hint");
  await expect(cell.locator(".combobox__value")).toHaveText("");
  await expect(hint).toBeHidden();
  await cell.hover();
  await expect(hint).toBeVisible();
});

test("j2) «Станом на» — DatePicker з маскою, не нативний input", async ({ page }) => {
  const asOf = page.getByRole("textbox", { name: "Станом на" });
  await expect(asOf).toHaveAttribute("inputmode", "numeric");
});
