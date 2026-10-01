// Точка входу (09-messaging.md §3.7): DB-міграції → NATS (стріми+KV) → WhatsApp-клієнт+pairing →
// pull-consumer NOTIFY_CMD → health-сервер. Watchdog: необроблений виняток у consumer-циклі не
// валить процес -- логується, цикл продовжується зі свіжим fetch (§3: "Chromium впав або завис
// → watchdog перезапускає клієнт; якщо не допомогло — процес завершується").

import { config } from "./config.ts";
import { sql } from "./db.ts";
import { runMigrations } from "./migrate.ts";
import { bindNotifyCmdConsumer, connectNats, type NatsHandles } from "./nats.ts";
import { createWhatsAppClient } from "./whatsapp/client.ts";
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
  waClient.initialize().catch((e: unknown) => {
    console.error("notifier: WhatsApp Client.initialize() провалився:", e);
  });

  startHealthServer(config.healthPort, () => nats);
  wireAdminCommands(nats.nc, pairing, channel);

  const consumer = await bindNotifyCmdConsumer(nats);
  console.log("notifier: consumer NOTIFY_CMD прив'язаний, готовий");

  // Технічний rate-limit (§3: "не більше N повідомлень на хвилину, випадкова пауза між
  // надсиланнями") -- мітигація ризику блокування номера, не продуктова вимога.
  const minGapMs = 60_000 / Math.max(config.rateLimitPerMinute, 1);

  for (;;) {
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
