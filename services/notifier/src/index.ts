// Точка входу (09-messaging.md §3.7): DB-міграції → NATS (стріми+KV) → WhatsApp-клієнт+pairing →
// pull-consumer NOTIFY_CMD → health-сервер. Watchdog: необроблений виняток у consumer-циклі не
// валить процес -- логується, цикл продовжується зі свіжим fetch (§3: "Chromium впав або завис
// → watchdog перезапускає клієнт; якщо не допомогло — процес завершується").

import { config } from "./config.ts";
import { sql } from "./db.ts";
import { runMigrations } from "./migrate.ts";
import { bindNotifyCmdConsumer, connectNats, publishStatus, type NatsHandles } from "./nats.ts";
import { createWhatsAppClient, purgeSession } from "./whatsapp/client.ts";
import { WhatsAppChannel } from "./whatsapp/channel.ts";
import { PairingStateMachine } from "./pairing.ts";
import { startHealthServer } from "./health.ts";
import { wireAdminCommands } from "./commands.ts";
import { handleOneMessage } from "./consumer.ts";

async function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function main() {
  console.log("notifier: застосовую міграції…");
  await runMigrations(sql);

  console.log("notifier: з'єдную з NATS…");
  const nats: NatsHandles = await connectNats();

  console.log("notifier: піднімаю WhatsApp-клієнт…");
  const waClient = createWhatsAppClient();
  const pairing = new PairingStateMachine(waClient, nats);
  const channel = new WhatsAppChannel(waClient);

  await publishStatus(nats, { state: "starting" });

  // Watchdog: якщо за 90с жодного івенту від whatsapp-web.js (qr/ready/auth_failure) —
  // сесія зависла (зіпсовані дані, Chromium не може стартувати). Видаляємо сесію і
  // виходимо — Docker автоматично перезапустить контейнер з чистим станом.
  const WATCHDOG_MS = 90_000;
  const watchdog = setTimeout(() => {
    console.error(
      `notifier: watchdog — жодного івенту за ${WATCHDOG_MS / 1000}с, ` +
        "видаляю сесію і завершую процес для перезапуску",
    );
    purgeSession();
    process.exit(1);
  }, WATCHDOG_MS);

  pairing.onFirstEvent = () => {
    clearTimeout(watchdog);
    console.log("notifier: watchdog скасовано — WhatsApp відповів");
  };

  waClient.initialize().catch((e: unknown) => {
    console.error("notifier: WhatsApp Client.initialize() провалився:", e);
    console.error("notifier: повідомлення залишаються в черзі NATS, буде повторна спроба при перезапуску");
    purgeSession();
  });

  startHealthServer(config.healthPort, () => nats);
  wireAdminCommands(nats.nc, pairing, channel);

  // KV TTL 60с — без heartbeat запис протухає і SSE показує "not_running".
  setInterval(async () => {
    try {
      await publishStatus(nats, pairing.getStatusSnapshot());
    } catch {}
  }, 45_000);

  const consumer = await bindNotifyCmdConsumer(nats);
  console.log("notifier: consumer NOTIFY_CMD прив'язаний, готовий");

  // Технічний rate-limit (§3: "не більше N повідомлень на хвилину, випадкова пауза між
  // надсиланнями") -- мітигація ризику блокування номера, не продуктова вимога.
  const minGapMs = 60_000 / Math.max(config.rateLimitPerMinute, 1);

  for (;;) {
    if (!channel.isReady()) {
      await sleep(5_000);
      continue;
    }
    try {
      const msgs = await consumer.fetch({ max_messages: 10, expires: 5_000 });
      for await (const msg of msgs) {
        await handleOneMessage(msg, nats, channel);
        await sleep(minGapMs + Math.random() * minGapMs * 0.5);
      }
    } catch (e) {
      console.error("notifier: помилка fetch-циклу consumer'а (watchdog продовжує):", e);
      await sleep(1_000);
    }
  }
}

main().catch((e) => {
  console.error("notifier: фатальна помилка старту:", e);
  process.exit(1);
});
