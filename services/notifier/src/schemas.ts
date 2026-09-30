// Рантайм-валідація (zod) на вході (09-messaging.md §2: "Невалідне повідомлення → одразу в DLQ
// з причиною, без повторів"). JSON Schema (`contracts/schema/*.json` → `src/generated/*.d.ts`)
// дає ТИП (compile-time), zod дає РАНТАЙМ-перевірку -- обидва мають описувати ту саму форму;
// розсинхрон між ними ловиться лише вручну (немає автоматичного JSON-Schema→zod генератора тут),
// тому за зміни в `contracts` МАЄ йти правка й тут (той самий принцип, що дублювання title-рядка
// в `contracts/tests/schema.rs` і `src/bin/gen_schema.rs`).

import { z } from "zod";

export const notifyTemplateSchema = z.enum(["discrepancy_detected"]);

export const notifySendSchema = z.object({
  recipient_org_id: z.number().int(),
  template: notifyTemplateSchema,
  dedupe_key: z.string().min(1),
});

export const envelopeSchema = <T extends z.ZodTypeAny>(payload: T) =>
  z.object({
    id: z.string(),
    type: z.string(),
    version: z.number().int(),
    occurred_at: z.string(),
    producer: z.string(),
    correlation_id: z.string(),
    causation_id: z.string().nullable().optional(),
    payload,
  });

export const notifySendEnvelopeSchema = envelopeSchema(notifySendSchema);
export type NotifySendEnvelope = z.infer<typeof notifySendEnvelopeSchema>;
