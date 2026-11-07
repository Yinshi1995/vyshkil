import { test, expect } from "@playwright/test";

/**
 * Модуль "особи поіменно" (`.claude/decisions/personnel-named-persons.md`): вимкнений за замовчуванням.
 * Тест сам визначає режим стенда за `GET /api/features`:
 *  - вимкнено → у інтерфейсі й мережі ЖОДНОГО сліду модуля (головна умова, доки нема юридичного погодження);
 *  - увімкнено (`PERSONNEL_ENABLED=1`) → повний сценарій: додати → побачити → дубль → зняти.
 * Стенд: `PASSWORD_LOGIN=1 DEV_AUTOLOGIN=admin`, вимислені дані (прізвище нижче — тестове).
 */
const LAST = "Е2ЕТестовий";

async function openFirstGroup(page: import("@playwright/test").Page) {
  await page.goto("/data");
  await page.locator("table tbody tr").first().click();
  await expect(page.getByText(/Прибуло:|План:|Розпочали:|Завершили:|Додано:/).first()).toBeVisible();
}

test("вимкнений модуль не лишає сліду в інтерфейсі й мережі", async ({ page, request }) => {
  const features = await (await request.get("/api/features")).json();
  test.skip(features.personnel === true, "на цьому стенді модуль увімкнено");

  await page.goto("/"); // автовхід виставляє cookie сесії
  const personnelRequests: string[] = [];
  page.on("request", (r) => {
    const u = r.url().replace(/^https?:\/\/[^/]+/, "");
    if (/\/api\/.*persons/.test(u)) personnelRequests.push(`${r.method()} ${u}`);
  });
  await openFirstGroup(page);
  await expect(page.getByTitle("Особи поіменно")).toHaveCount(0);
  expect(personnelRequests).toEqual([]);

  // і прямий виклик API — 404, навіть на сміттєве тіло (не видає існування ендпоінта)
  expect((await page.request.get("/api/data/events/1/persons")).status()).toBe(404);
  const post = await page.request.post("/api/data/events/1/persons", { data: "{broken", headers: { "content-type": "application/json" } });
  expect(post.status()).toBe(404);
});

test("увімкнений модуль: додати, побачити, дубль, зняти", async ({ page, request }) => {
  const features = await (await request.get("/api/features")).json();
  test.skip(features.personnel !== true, "модуль вимкнено (дефолт) — сценарій не застосовний");

  await openFirstGroup(page);
  await page.getByTitle("Особи поіменно").first().click();
  const panel = page.getByTestId("event-persons");
  await expect(panel).toBeVisible();

  const fill = async () => {
    await page.getByPlaceholder("Звання").fill("сержант");
    await page.getByPlaceholder("Прізвище").fill(LAST);
    await page.getByPlaceholder("Ім'я").fill("Іван");
    await page.getByPlaceholder("Дата народження").fill("17.05.1990");
    await page.getByRole("button", { name: "Зберегти" }).click();
  };
  await panel.getByRole("button", { name: "Додати" }).click();
  await fill();
  await expect(page.getByTestId("event-person")).toHaveCount(1);
  await expect(page.getByTestId("event-person").first()).toContainText(LAST);
  await expect(page.getByTestId("event-person").first()).toContainText("17.05.1990");

  // ПІБ не потрапляє в URL і localStorage
  expect(page.url()).not.toContain(LAST);
  expect(await page.evaluate(() => JSON.stringify(localStorage))).not.toContain(LAST);

  // дубль у ту саму подію — помилка, список не росте
  await panel.getByRole("button", { name: "Додати" }).click();
  await fill();
  await expect(page.locator("[data-sonner-toast]").first()).toContainText("вже додана");
  await page.getByRole("button", { name: "Скасувати" }).click();
  await expect(page.getByTestId("event-person")).toHaveCount(1);

  // зняти — і прибрати за собою
  await page.getByTitle("Зняти з події").first().click();
  await expect(page.getByTestId("event-person")).toHaveCount(0);
});
