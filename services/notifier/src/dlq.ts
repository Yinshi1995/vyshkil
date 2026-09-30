// DLQ (09-messaging.md §3.3, §6: "невалідне (по zod) повідомлення → одразу в DLQ з причиною,
// без повторів"; "max_deliver → DLQ"). Раніше ці два випадки просто губили повідомлення
// (`msg.term()`/нескінченний `nak()` без жодного сліду) -- реальна прогалина, знайдена при
// побудові admin-екрана "Черги" (Фаза 4): без фактичного DLQ-стріму кнопці "повторити" нема
// що показувати.

import type { NatsHandles } from "./nats.ts";

export const DLQ_SUBJECT = "vyshkil.dlq.notify.send.v1";

export interface DlqEntry {
  original_subject: string;
  reason: string;
  payload_base64: string;
  failed_at: string;
}

export async function publishToDlq(
  { js }: NatsHandles,
  originalSubject: string,
  payload: Uint8Array,
  reason: string,
): Promise<void> {
  const entry: DlqEntry = {
    original_subject: originalSubject,
    reason,
    payload_base64: Buffer.from(payload).toString("base64"),
    failed_at: new Date().toISOString(),
  };
  await js.publish(DLQ_SUBJECT, new TextEncoder().encode(JSON.stringify(entry)));
}
