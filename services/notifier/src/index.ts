// Точка входу (09-messaging.md §3.7): DB-міграції → NATS (стріми+KV) → WhatsApp-клієнт+pairing →
// pull-consumer NOTIFY_CMD → health-сервер. Watchdog: необроблений виняток у consumer-циклі не
// валить процес -- логується, цикл продовжується зі свіжим fetch (§3: "Chromium впав або завис
// → watchdog перезапускає клієнт; якщо не допомогло — процес завершується").

import { config } from "./config.ts";
import { sql } from "./db.ts";
import { runMigrations } from "./migrate.ts";
import {
  bindNotifyCmdConsumer,
  connectNats,
  MAX_DELIVER,
  publishNotifyResult,
  type NatsHandles,
} from "./nats.ts";
import { processNotifySend } from "./delivery.ts";
import { createWhatsAppClient } from "./whatsapp/client.ts";
import { WhatsAppChannel } from "./whatsapp/channel.ts";
import { PairingStateMachine } from "./pairing.ts";
import { startHealthServer } from "./health.ts";
import { wireAdminCommands } from "./commands.ts";
import { publishToDlq } from "./dlq.ts";

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
        const messageId = msg.headers?.get("Nats-Msg-Id") ?? String(msg.seq);
        try {
          const outcome = await processNotifySend(msg.data, messageId, channel);
          if (outcome.kind === "invalid") {
            // §6: "невалідне (по zod) повідомлення → одразу в DLQ з причиною, без повторів" --
            // не просто `term()` (те мовчки губило повідомлення, реальна прогалина, знайдена
            // при побудові admin-екрана "Черги": без запису в DLQ нема що показувати/повторити).
            console.error("notifier: невалідне повідомлення, DLQ (без повторів):", outcome.reason);
            await publishToDlq(nats, "vyshkil.notify.send.v1", msg.data, outcome.reason);
            msg.term();
            continue;
          }
          if (outcome.kind === "duplicate") {
            msg.ack();
            continue;
          }
          const resultEnvelope = {
            id: crypto.randomUUID(),
            type: "vyshkil.notify.result.v1",
            version: 1,
            occurred_at: new Date().toISOString(),
            producer: "notifier",
            correlation_id: crypto.randomUUID(),
            causation_id: null,
            payload: {
              recipient_org_id: outcome.orgId,
              status: outcome.status,
              dedupe_key: outcome.dedupeKey,
            },
          };
          await publishNotifyResult(nats, resultEnvelope, `result-${messageId}`);
          msg.ack();
          await sleep(minGapMs + Math.random() * minGapMs * 0.5);
        } catch (e) {
          // §6: "max_deliver → DLQ". `msg.info.deliveryCount` -- скільки разів JetStream УЖЕ
          // доставляв цей msg (рахуючи поточну спробу); на ОСТАННІЙ дозволеній спробі -- в DLQ й
          // term (не nak, інакше consumer просто тихо перестане доставляти далі без жодного
          // сліду). Доти -- звичайний nak із затримкою (транзиєнтна помилка, напр. Chromium
          // тимчасово недоступний -- команда чекає, а не губиться, §3: "команди в черзі...
          // чекають").
          if (msg.info.deliveryCount >= MAX_DELIVER) {
            console.error(`notifier: вичерпано ${MAX_DELIVER} спроб, DLQ:`, e);
            await publishToDlq(nats, "vyshkil.notify.send.v1", msg.data, String(e));
            msg.term();
          } else {
            console.error(`notifier: обробка команди провалилась (спроба ${msg.info.deliveryCount}/${MAX_DELIVER}), nak:`, e);
            msg.nak(5_000);
          }
        }
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
