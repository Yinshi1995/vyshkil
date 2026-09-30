// Обробка однієї команди `NotifySend` (09-messaging.md §3.2, §3.7): валідація (zod) → inbox-
// перевірка → рендер шаблону → надсилання кожному активному контакту організації → журнал →
// позначення оброблено. Усе, крім самого `channel.send` (зовнішній виклик), -- в одній
// транзакції.

import { contactsForOrg, insertDeliveryLog, isAlreadyProcessed, markProcessed, sql } from "./db.ts";
import { maskPhone } from "./mask.ts";
import { notifySendEnvelopeSchema } from "./schemas.ts";
import { renderTemplate } from "./templates.ts";
import type { Channel } from "./whatsapp/channel.ts";

export type ProcessOutcome =
  | { kind: "invalid"; reason: string }
  | { kind: "duplicate" }
  | { kind: "processed"; orgId: number; dedupeKey: string; status: "delivered" | "failed" | "suppressed" };

export async function processNotifySend(
  rawPayload: Uint8Array,
  messageId: string,
  channel: Channel,
): Promise<ProcessOutcome> {
  let parsed: ReturnType<typeof notifySendEnvelopeSchema.parse>;
  try {
    const json = JSON.parse(new TextDecoder().decode(rawPayload));
    parsed = notifySendEnvelopeSchema.parse(json);
  } catch (e) {
    return { kind: "invalid", reason: String(e) };
  }

  if (await isAlreadyProcessed(messageId)) {
    return { kind: "duplicate" };
  }

  const { recipient_org_id: orgId, template, dedupe_key: dedupeKey } = parsed.payload;
  const text = renderTemplate(template);
  const contacts = await contactsForOrg(orgId);

  let anyFailed = false;

  await sql.begin(async (tx) => {
    for (const contact of contacts) {
      const result = await channel.send(contact.phone, text);
      if (result.status !== "delivered") anyFailed = true;
      await insertDeliveryLog(tx, {
        orgId,
        phoneMasked: maskPhone(contact.phone),
        template,
        status: result.status,
        error: result.error,
      });
    }
    await markProcessed(tx, messageId);
  });

  // Немає активних контактів -- нічого фізично не надсилалось, це не помилка каналу.
  // Будь-яка невдача серед контактів (навіть часткова) -- "failed", NotifyStatus не має
  // проміжного "частково" стану.
  const status = contacts.length === 0 ? "suppressed" : anyFailed ? "failed" : "delivered";

  return { kind: "processed", orgId, dedupeKey, status };
}
