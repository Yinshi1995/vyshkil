// Побудова реального `whatsapp-web.js` Client (09 §3, §4) -- сесія (`LocalAuth`) в окремому
// іменованому томі (docker-compose), права лише для користувача сервісу. `executablePath` --
// системний Chromium у контейнері (§3: "Puppeteer вказує на нього, свій Chromium при install не
// качає"), НЕ puppeteer-власний завантажений білд.

import pkg from "whatsapp-web.js";
const { Client, LocalAuth } = pkg;
import { config } from "../config.ts";

export function createWhatsAppClient(): InstanceType<typeof Client> {
  const args = ["--disable-dev-shm-usage"];
  if (config.waDisableSandbox) {
    // §3: "якщо без --no-sandbox не стартує — дай seccomp-профіль для Chromium, а не вимикай
    // sandbox мовчки" -- тому логуємо голосно, а не тихо, кожен раз, коли ця env-змінна активна.
    console.warn(
      "WA_DISABLE_SANDBOX=true -- Chromium запускається БЕЗ sandbox. " +
        "Це тимчасовий обхід, не рішення за замовчуванням -- дивись .claude/decisions/" +
        "notifier-ts-whatsapp-web-js.md про seccomp-профіль як правильну альтернативу.",
    );
    args.push("--no-sandbox", "--disable-setuid-sandbox");
  }

  return new Client({
    authStrategy: new LocalAuth({ dataPath: config.waSessionPath }),
    puppeteer: {
      headless: true,
      executablePath: config.waPuppeteerExecutablePath || undefined,
      args,
    },
  });
}
