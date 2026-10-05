// Обробка ОДНОГО повідомлення з NOTIFY_CMD (09 §6: "повтор після збою провайдера",
// "needs_pairing → повідомлення чекають", "max_deliver → DLQ") -- винесено з `index.ts`'s циклу
// в окрему функцію САМЕ заради тестованості: мінімальний інтерфейс `ConsumableMessage` (не
// реальний `async-nats` тип) дозволяє тестам давати легкий fake замість живого JetStream-
// повідомлення.

import { ChannelNotReadyError, type Channel } from "./whatsapp/channel.ts";
import { processNotifySend } from "./delivery.ts";
import { publishNotifyResult, MAX_DELIVER, type NatsHandles } from "./nats.ts";
import { publishToDlq } from "./dlq.ts";

export interface ConsumableMessage {
  data: Uint8Array;
  info: { deliveryCount: number };
  headers?: { get(name: string): string | undefined } | undefined;
  seq: number;
  ack(): void;
  nak(delayMs?: number): void;
  term(): void;
}

/** Технічний rate-limit (§3) -- викликач (`index.ts`) сам вирішує, скільки спати МІЖ
 *  повідомленнями; ця функція обробляє рівно одне, без сну всередині -- тестовіше. */
export async function handleOneMessage(
  msg: ConsumableMessage,
  nats: NatsHandles,
  channel: Channel,
): Promise<void> {
  const messageId = msg.headers?.get("Nats-Msg-Id") ?? String(msg.seq);
  try {
    const outcome = await processNotifySend(msg.data, messageId, channel);
    if (outcome.kind === "invalid") {
      // §6: "невалідне (по zod) повідомлення → одразу в DLQ з причиною, без повторів".
      console.error("notifier: невалідне повідомлення, DLQ (без повторів):", outcome.reason);
      await publishToDlq(nats, "vyshkil.notify.send.v1", msg.data, outcome.reason);
      msg.term();
      return;
    }
    if (outcome.kind === "duplicate") {
      msg.ack();
      return;
    }
    if (outcome.status === "failed") {
      if (msg.info.deliveryCount >= MAX_DELIVER) {
        console.error(`notifier: доставка провалювалась ${MAX_DELIVER} спроб, DLQ`);
        await publishToDlq(nats, "vyshkil.notify.send.v1", msg.data, "delivery failed after max retries");
        msg.term();
      } else {
        console.warn(`notifier: доставка провалилась, nak 30с (спроба ${msg.info.deliveryCount}/${MAX_DELIVER})`);
        msg.nak(30_000);
      }
      return;
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
  } catch (e) {
    // §6: "max_deliver → DLQ" / "needs_pairing → повідомлення чекають" -- те саме `nak`, що
    // покриває ОБИДВА випадки: WhatsApp-клієнт ще не прив'язаний (`channel.send` кидає) --
    // команда чекає (nak), доки прив'язка не відновиться й наступна спроба не вдасться; на
    // ОСТАННІЙ дозволеній спробі -- здаємось, в DLQ (не губимо мовчки).
    if (e instanceof ChannelNotReadyError) {
      console.warn(`notifier: канал не готовий, nak 30с (спроба ${msg.info.deliveryCount}/${MAX_DELIVER})`);
      msg.nak(30_000);
    } else if (msg.info.deliveryCount >= MAX_DELIVER) {
      console.error(`notifier: вичерпано ${MAX_DELIVER} спроб, DLQ:`, e);
      await publishToDlq(nats, "vyshkil.notify.send.v1", msg.data, String(e));
      msg.term();
    } else {
      console.error(`notifier: обробка команди провалилась (спроба ${msg.info.deliveryCount}/${MAX_DELIVER}), nak:`, e);
      msg.nak(5_000);
    }
  }
}
