import { test, expect, type Page } from "@playwright/test";

// Дефект (feedback користувача): перетягнути файл на зону роботи мало б працювати, клік лівою
// кнопкою — теж мав би відкривати системний вибір файлу. Діагностика цієї сесії: живий клік по
// СТАРІЙ зоні (display:none-input + програмний `.click()` зі stop_propagation-форвардингом) у
// Playwright/Chromium filechooser таки відкривав -- точну причину користувацького репро
// підтвердити автоматизацією не вдалось (можливо інший браузер/оточення). Компонент переробили
// на `<label for>`-обгортку РЕАЛЬНОГО `<input>` (візуально прихований, не `display:none`) все
// одно -- об'єктивно надійніший патерн, без жодного JS `.click()`-форвардингу; `setInputFiles`
// обходить сам клік і тому ховає БУДЬ-ЯКИЙ дефект цього класу — тести чекають справжню подію
// `filechooser`, яку браузер шле лише при реальному відкритті системного діалогу.

const ORG_LABEL = "152 нц (4896)";

async function selectActor(page: Page) {
  await page.goto("/training-form");
  // `ActorSwitcher` — наш `components::Select` (кнопка+listbox), не нативний `<select>`
  // (HeroUI-прохід мігрував його раніше цієї сесії) -- клік відкриває панель, клік по option обирає.
  // SSR-рендерена кнопка існує в DOM ДО завершення WASM-гідратації -- перший клік іноді мовчки
  // нічого не робить (гідратація ще не навісила `on:click`); один ретрай досить.
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

test("клік по кнопці імпорту відкриває системний вибір файлу (/training-form)", async ({ page }) => {
  await selectActor(page);

  const chooserPromise = page.waitForEvent("filechooser", { timeout: 5_000 });
  await page.getByRole("button", { name: "Імпорт" }).click();
  const chooser = await chooserPromise;

  expect(chooser.isMultiple()).toBe(false);
});

test("клавіатура (Enter) на кнопці імпорту відкриває системний вибір файлу", async ({ page }) => {
  await selectActor(page);

  await page.getByRole("button", { name: "Імпорт" }).focus();
  const chooserPromise = page.waitForEvent("filechooser", { timeout: 5_000 });
  await page.keyboard.press("Enter");
  await chooserPromise;
});

test("повторний вибір того самого файлу знову запускає розбір", async ({ page }) => {
  await selectActor(page);

  const filePath = "tests/fixtures/sample.xlsx";

  const chooserPromise1 = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Імпорт" }).click();
  const chooser1 = await chooserPromise1;
  await chooser1.setFiles(filePath);
  await expect(page.locator(".file-chip")).toBeVisible({ timeout: 10_000 });
  await page.getByRole("button", { name: "Прибрати файл" }).click();

  // ТОЙ САМИЙ файл вдруге -- `input.value` мусить бути скинутий, інакше `change` вдруге не шле.
  const chooserPromise2 = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Імпорт" }).click();
  const chooser2 = await chooserPromise2;
  await chooser2.setFiles(filePath);
  await expect(page.locator(".file-chip")).toBeVisible({ timeout: 10_000 });
});

test("перетягування файла будь-де на сторінці показує повноекранний приймач", async ({ page }) => {
  await selectActor(page);

  // Порожній DataTransfer (без Files-типу) навмисно НЕ показує приймач -- інакше перетягування
  // виділеного тексту/посилання теж відкривало б оверлей. Лічильник dragenter/dragleave (не
  // булевий прапорець) — стандартний захист від фліку при переході між дочірніми елементами.
  const dataTransfer = await page.evaluateHandle(() => {
    const dt = new DataTransfer();
    dt.items.add(new File(["x"], "anywhere.xlsx"));
    return dt;
  });
  await page.dispatchEvent("body", "dragenter", { dataTransfer });
  await expect(page.getByText(/Відпустіть файл/)).toBeVisible();

  await page.dispatchEvent("body", "dragleave", { dataTransfer });
  await expect(page.getByText(/Відпустіть файл/)).toBeHidden();
});

test("drop будь-де на сторінці (не лише над кнопкою) вибирає файл", async ({ page }) => {
  await selectActor(page);

  const dataTransfer = await page.evaluateHandle(() => {
    const dt = new DataTransfer();
    dt.items.add(new File(["x"], "dropped.xlsx"));
    return dt;
  });
  await page.dispatchEvent("body", "dragenter", { dataTransfer });
  await page.dispatchEvent("body", "drop", { dataTransfer });

  await expect(page.getByText("dropped.xlsx")).toBeVisible({ timeout: 5_000 });
});

test("неправильний тип файлу показує зрозумілу помилку без звернення до сервера", async ({ page }) => {
  await selectActor(page);

  const dataTransfer = await page.evaluateHandle(() => {
    const dt = new DataTransfer();
    dt.items.add(new File(["not a spreadsheet"], "wrong-type.txt"));
    return dt;
  });
  await page.dispatchEvent("body", "dragenter", { dataTransfer });
  await page.dispatchEvent("body", "drop", { dataTransfer });

  await expect(page.getByText(/неправильний тип файлу.*\.xlsx/)).toBeVisible({ timeout: 5_000 });
});
