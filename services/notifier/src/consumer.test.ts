// Інтеграційні сценарії §6: "повтор після збою провайдера" і "needs_pairing → повідомлення
// чекають" -- обидва проходять через ОДИН і той самий механізм (`channel.send` кидає →
// nak-до-max_deliver), тому тестуються тут разом із FakeChannel, що навмисно кидає задану
// кількість разів перед успіхом (симулює "клієнт ще не прив'язаний, потім прив'язався").
// Реальний Postgres (NOTIFIER_DATABASE_URL) потрібен -- delivery.ts сам пише в БД.

import { strict as assert } from "node:assert";
import { test } from "node:test";
import { handleOneMessage, type ConsumableMessage } from "./consumer.ts";
import { sql } from "./db.ts";
import type { Channel, SendResult } from "./whatsapp/channel.ts";
import type { NatsHandles } from "./nats.ts";

const hasDb = !!process.env.NOTIFIER_DATABASE_URL;
if (!hasDb) {
  console.error("NOTIFIER_DATABASE_URL не задано — consumer.test.ts пропущено");
}

const testOrgId = 900_101;

function envelope(dedupeKey: string) {
  return {
    id: "00000000-0000-7000-8000-000000000001",
    type: "vyshkil.notify.send.v1",
    version: 1,
    occurred_at: new Date().toISOString(),
    producer: "test",
    correlation_id: "00000000-0000-7000-8000-000000000002",
    causation_id: null,
    payload: { recipient_org_id: testOrgId, template: "discrepancy_detected", dedupe_key: dedupeKey },
  };
}

/** Мінімальний fake NATS-handles -- `publishNotifyResult`/`publishToDlq` з `nats.ts` зовсім не
 *  використовують `nc`/`jsm`/`kv` напряму в тестованих гілках коду тут (нема DLQ/success-шляху
 *  в "очікує прив'язки"-сценарії нижче), тому достатньо підставного `js` з no-op publish. */
function fakeNats(onPublish?: (subject: string, data: Uint8Array) => void): NatsHandles {
  return {
    nc: {} as never,
    jsm: {} as never,
    kv: {} as never,
    js: {
      publish: async (subject: string, data: Uint8Array) => {
        onPublish?.(subject, data);
        return {} as never;
      },
    } as never,
  };
}

function fakeMessage(
  data: Uint8Array,
  deliveryCount: number,
  messageId: string,
): ConsumableMessage & { calls: string[] } {
  const calls: string[] = [];
  return {
    data,
    info: { deliveryCount },
    seq: deliveryCount,
    headers: { get: (name: string) => (name === "Nats-Msg-Id" ? messageId : undefined) },
    ack: () => calls.push("ack"),
    nak: (delayMs?: number) => calls.push(`nak:${delayMs}`),
    term: () => calls.push("term"),
    calls,
  };
}

/** `channel.send` кидає рівно `failTimes` разів (симулює "WhatsApp ще не прив'язаний" або
 *  "провайдер тимчасово недоступний"), потім починає успішно доставляти -- саме так на практиці
 *  виглядає і needs_pairing-очікування, і звичайний transient-збій. */
class FlakyChannel implements Channel {
  private remaining: number;
  public sentCount = 0;

  constructor(failTimes: number) {
    this.remaining = failTimes;
  }

  isReady(): boolean {
    return true;
  }

  async send(_phone: string, _text: string): Promise<SendResult> {
    if (this.remaining > 0) {
      this.remaining -= 1;
      throw new Error("needs_pairing: WhatsApp-клієнт ще не прив'язаний");
    }
    this.sentCount += 1;
    return { status: "delivered" };
  }
}

test("retry after transient failure: nak below max_deliver, succeeds once channel recovers", { skip: !hasDb }, async (t) => {
  await sql`DELETE FROM org_contact WHERE org_id = ${testOrgId}`;
  await sql`DELETE FROM inbox WHERE message_id LIKE 'consumer-test-%'`;
  await sql`INSERT INTO org_contact (org_id, phone, channel) VALUES (${testOrgId}, '+380501111111', 'whatsapp')`;

  await t.test("deliveryCount=1 (below max_deliver): nak, not DLQ'd, not sent", async () => {
    const channel = new FlakyChannel(3);
    const msg = fakeMessage(new TextEncoder().encode(JSON.stringify(envelope("retry-1"))), 1, "consumer-test-retry");
    await handleOneMessage(msg, fakeNats(), channel);
    assert.deepEqual(msg.calls, ["nak:5000"]);
    assert.equal(channel.sentCount, 0);
  });

  await t.test("once the channel recovers, the SAME message id is delivered and acked", async () => {
    // Симулює "наступна доставка того самого msg" (JetStream саме так і робить після nak) --
    // той самий message_id (inbox-ключ), канал уже "прив'язаний" (не кидає).
    const channel = new FlakyChannel(0);
    // ТОЙ САМИЙ message_id, що в попередньому підтесті -- симулює реальну поведінку JetStream:
    // redelivery того самого повідомлення зберігає Nats-Msg-Id, лише deliveryCount зростає.
    const msg = fakeMessage(new TextEncoder().encode(JSON.stringify(envelope("retry-1"))), 2, "consumer-test-retry");
    let published: string | null = null;
    await handleOneMessage(msg, fakeNats((subject) => { published = subject; }), channel);
    assert.deepEqual(msg.calls, ["ack"]);
    assert.equal(channel.sentCount, 1);
    assert.equal(published, "vyshkil.notify.result.v1");
  });
});

test("exhausted retries (deliveryCount >= max_deliver): DLQ + term, not nak forever", { skip: !hasDb }, async () => {
  await sql`DELETE FROM org_contact WHERE org_id = ${testOrgId}`;
  await sql`INSERT INTO org_contact (org_id, phone, channel) VALUES (${testOrgId}, '+380502222222', 'whatsapp')`;

  const channel = new FlakyChannel(999); // завжди кидає -- симулює постійно недоступний канал
  const msg = fakeMessage(
    new TextEncoder().encode(JSON.stringify(envelope("exhausted-1"))),
    20, // = MAX_DELIVER
    "consumer-test-exhausted",
  );
  let dlqSubject: string | null = null;
  await handleOneMessage(msg, fakeNats((subject) => { dlqSubject = subject; }), channel);

  assert.deepEqual(msg.calls, ["term"], "на вичерпаній спробі -- term, НЕ нескінченний nak");
  assert.equal(dlqSubject, "vyshkil.dlq.notify.send.v1");
});

test("failed delivery (status:'failed'): nak below max_deliver, DLQ+term on exhaustion", { skip: !hasDb }, async (t) => {
  await sql`DELETE FROM org_contact WHERE org_id = ${testOrgId}`;
  await sql`DELETE FROM inbox WHERE message_id LIKE 'consumer-test-fail-%'`;
  await sql`INSERT INTO org_contact (org_id, phone, channel) VALUES (${testOrgId}, '+380503333333', 'whatsapp')`;

  const failChannel: Channel = {
    isReady: () => true,
    send: async () => ({ status: "failed" as const, error: "group not found" }),
  };

  await t.test("deliveryCount=1: nak 30s (retry), message stays in queue", async () => {
    const msg = fakeMessage(
      new TextEncoder().encode(JSON.stringify(envelope("fail-delivery-1"))),
      1,
      "consumer-test-fail-1",
    );
    await handleOneMessage(msg, fakeNats(), failChannel);
    assert.deepEqual(msg.calls, ["nak:30000"]);
  });

  await t.test("deliveryCount=MAX_DELIVER: DLQ + term", async () => {
    const msg = fakeMessage(
      new TextEncoder().encode(JSON.stringify(envelope("fail-delivery-2"))),
      20,
      "consumer-test-fail-2",
    );
    let dlqSubject: string | null = null;
    await handleOneMessage(msg, fakeNats((subject) => { dlqSubject = subject; }), failChannel);
    assert.deepEqual(msg.calls, ["term"]);
    assert.equal(dlqSubject, "vyshkil.dlq.notify.send.v1");
  });

  await sql.end();
});
