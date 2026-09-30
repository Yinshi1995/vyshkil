// Мінімальна плоска конфігурація з env — той самий підхід, що `server/src/config.rs` (жодного
// окремого config-фреймворку заради кількох плоских значень).

function required(name: string): string {
  const v = process.env[name];
  if (!v) throw new Error(`${name} must be set (see .env.example)`);
  return v;
}

function optional(name: string, fallback: string): string {
  return process.env[name] ?? fallback;
}

export const config = {
  databaseUrl: required("NOTIFIER_DATABASE_URL"),
  natsUrl: optional("NATS_URL", "nats://127.0.0.1:4222"),
  // Сесія WhatsApp (LocalAuth) -- окремий іменований том у docker-compose (§3: "повний доступ
  // до акаунта", не в git/бекапи).
  waSessionPath: optional("WA_SESSION_PATH", "./wa-session"),
  waPuppeteerExecutablePath: optional("WA_PUPPETEER_EXECUTABLE_PATH", ""),
  // За замовчуванням sandbox УВІМКНЕНО (§3: "не вимикай sandbox мовчки") -- вимкнення лише явною
  // env-змінною, з голосним попередженням у логах (index.ts).
  waDisableSandbox: optional("WA_DISABLE_SANDBOX", "false") === "true",
  waPrintQrToLogs: optional("WA_PRINT_QR_TO_LOGS", "0") === "1",
  // Технічний rate-limit (§3: "не більше N повідомлень на хвилину, випадкова пауза") --
  // мітигація ризику блокування номера, не продуктова вимога.
  rateLimitPerMinute: Number(optional("WA_RATE_LIMIT_PER_MINUTE", "20")),
  healthPort: Number(optional("HEALTH_PORT", "8080")),
};
