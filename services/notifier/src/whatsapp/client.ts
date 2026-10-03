// Побудова реального `whatsapp-web.js` Client (09 §3, §4) -- сесія (`LocalAuth`) в окремому
// іменованому томі (docker-compose), права лише для користувача сервісу. `executablePath` --
// системний Chromium у контейнері (§3: "Puppeteer вказує на нього, свій Chromium при install не
// качає"), НЕ puppeteer-власний завантажений білд.

import fs from "node:fs";
import path from "node:path";
import pkg from "whatsapp-web.js";
const { Client, LocalAuth } = pkg;
import { config } from "../config.ts";

function clearStaleLocks(): void {
  const sessionDir = path.join(config.waSessionPath, "session");
  for (const name of ["SingletonLock", "SingletonCookie", "SingletonSocket"]) {
    const p = path.join(sessionDir, name);
    try {
      fs.unlinkSync(p);
    } catch {}
  }
}

export function purgeSession(): void {
  const sessionDir = path.join(config.waSessionPath, "session");
  try {
    fs.rmSync(sessionDir, { recursive: true, force: true });
    console.log("notifier: сесію WhatsApp повністю видалено");
  } catch (e) {
    console.error("notifier: не вдалося видалити сесію:", e);
  }
}

export function createWhatsAppClient(): InstanceType<typeof Client> {
  clearStaleLocks();

  const args = ["--disable-dev-shm-usage"];
  if (config.waDisableSandbox) {
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
