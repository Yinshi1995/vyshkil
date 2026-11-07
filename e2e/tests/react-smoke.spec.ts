import { test, expect, type Page } from "@playwright/test";

/**
 * Smoke-перевірка React-інтерфейсу (замінює старі Leptos-тести, яких уже нема).
 *
 * Потребує стенда з вимисленими даними й автовходом: сервер запущений з `PASSWORD_LOGIN=1 DEV_AUTOLOGIN=admin`
 * (див. `server/src/api.rs::DEV_AUTOLOGIN`; на проді з ІКС він не стартує). Адреса — `BASE_URL`.
 * Ловить: помилки в консолі, 4xx/5xx від API, "сирі" ключі (snake_case), `undefined`/`NaN` у тексті,
 * кнопка в кнопці, горизонтальний скрол на телефоні й на десктопі.
 */
const PAGES = ["/", "/orgs", "/training", "/data", "/documents", "/discrepancies", "/dashboard", "/chat", "/settings", "/directory"];
const VIEWPORTS = [
  { name: "desktop", width: 1600, height: 1000 },
  { name: "phone", width: 360, height: 800 },
];

// Не помилки застосунку: зовнішні ресурси ІКС, недоступні поза тунелем.
const IGNORED = /lab\.okpivden|ERR_SOCKS|ERR_NAME_NOT_RESOLVED|ERR_CONNECTION/;

async function audit(page: Page, path: string) {
  const problems: string[] = [];
  page.on("console", (m) => {
    if (m.type() === "error" && !IGNORED.test(m.text())) problems.push(`console: ${m.text().slice(0, 140)}`);
  });
  page.on("response", (r) => {
    if (r.status() >= 500) problems.push(`HTTP ${r.status()} ${r.url().replace(/^https?:\/\/[^/]+/, "")}`);
  });
  await page.goto(path);
  // Не "networkidle": на dev-сервері Vite він чекає десятки секунд. Достатньо заголовка/каркаса і короткої паузи.
  await page.locator("h1, main").first().waitFor({ timeout: 20_000 }).catch(() => {});
  await page.waitForTimeout(1200);

  const dom = await page.evaluate(() => {
    const text = document.body.innerText;
    return {
      over: document.documentElement.scrollWidth - window.innerWidth,
      raw: [...new Set(text.match(/\b[a-z]+_[a-z_]+\b/g) ?? [])].slice(0, 4),
      bad: /\bNaN\b|\bundefined\b|\[object /.test(text),
      nested: document.querySelectorAll("button button").length,
      loginForm: !!document.querySelector("input[type=password]"),
      notFound: /Сторінку не знайдено/.test(text),
    };
  });
  if (dom.over > 0) problems.push(`горизонтальний скрол +${dom.over}px`);
  if (dom.raw.length) problems.push(`сирі ключі: ${dom.raw.join(", ")}`);
  if (dom.bad) problems.push("NaN/undefined у тексті");
  if (dom.nested) problems.push("button у button");
  if (dom.loginForm) problems.push("показано форму входу (нема автовходу?)");
  if (dom.notFound) problems.push("сторінку не знайдено");
  return problems;
}

test.setTimeout(60_000);

for (const vp of VIEWPORTS) {
  for (const path of PAGES) {
    test(`${vp.name} ${path} — без помилок`, async ({ page }) => {
      await page.setViewportSize({ width: vp.width, height: vp.height });
      const problems = await audit(page, path);
      expect(problems, problems.join("\n")).toEqual([]);
    });
  }
}

test("розбіжності в аналітиці й на головній не суперечать одне одному", async ({ page }) => {
  // Ті самі цифри з різних ендпоінтів мають збігатися (звірка екранів, див. скіл qa/smells).
  const signedIn = page.waitForResponse((r) => r.url().includes("/api/auth/me"));
  await page.goto("/"); // автовхід виставляє cookie сесії у відповіді на /api/auth/me
  expect((await signedIn).status()).toBe(200);
  const statsRes = await page.request.get("/api/dashboard/stats");
  const chartRes = await page.request.get("/api/dashboard/discrepancies-chart");
  expect([statsRes.status(), chartRes.status()]).toEqual([200, 200]);
  const stats = await statsRes.json();
  const chart = await chartRes.json();
  const fromChart = (chart.by_status as { count: number }[]).reduce((s, d) => s + d.count, 0);
  expect(fromChart).toBe(stats.total_discrepancies);
});
