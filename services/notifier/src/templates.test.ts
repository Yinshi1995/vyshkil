import { strict as assert } from "node:assert";
import { test } from "node:test";
import { renderTemplate } from "./templates.ts";

// Регресійний тест на приватність (09 §3.5 + 04 §5): жоден шаблон не містить підстановки
// (`{`/`%s`/`${`) -- сам факт наявності підстановки означав би, що десь вставляється доменне
// значення в текст, якого сюди НЕ можна передати (брокер несе лише enum/ідентифікатори).
test("templates contain no substitution placeholders", () => {
  const texts = [renderTemplate("discrepancy_detected")];
  for (const text of texts) {
    assert.ok(!text.includes("{"), `шаблон містить "{": ${text}`);
    assert.ok(!text.includes("%s"), `шаблон містить "%s": ${text}`);
    assert.ok(!text.includes("${"), `шаблон містить "\${": ${text}`);
  }
});

test("discrepancy_detected template matches 04 §5 exact wording", () => {
  assert.equal(
    renderTemplate("discrepancy_detected"),
    "Виявлено розбіжності у ваших даних щодо підготовки. Увійдіть у систему та перевірте розділ \"Розбіжності\".",
  );
});
