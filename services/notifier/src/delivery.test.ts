// Інтеграційний тест (09-messaging.md §6): реальний Postgres (NOTIFIER_DATABASE_URL), FakeChannel
// замість реального WhatsApp-клієнта -- "доставка рівно раз (inbox)" + "невалідне → DLQ без
// повторів". Без NOTIFIER_DATABASE_URL -- пропущено (той самий підхід, що Rust-бік:
// TEST_DATABASE_URL/TEST_NATS_URL).

import { strict as assert } from "node:assert";
import { test, type TestContext } from "node:test";
import { processNotifySend } from "./delivery.ts";
import { sql } from "./db.ts";
import { FakeChannel } from "./whatsapp/channel.ts";

const hasDb = !!process.env.NOTIFIER_DATABASE_URL;
if (!hasDb) {
  console.error(
    "NOTIFIER_DATABASE_URL не задано — інтеграційний тест delivery.test.ts пропущено",
  );
}

const testOrgId = 900_001;

function envelope(overrides: Partial<{ recipient_org_id: number; template: string; dedupe_key: string }> = {}) {
  return {
    id: "00000000-0000-7000-8000-000000000001",
    type: "vyshkil.notify.send.v1",
    version: 1,
    occurred_at: new Date().toISOString(),
    producer: "test",
    correlation_id: "00000000-0000-7000-8000-000000000002",
    causation_id: null,
    payload: {
      recipient_org_id: testOrgId,
      template: "discrepancy_detected",
      dedupe_key: "test-dedupe",
      ...overrides,
    },
  };
}

test("delivery pipeline", { skip: !hasDb }, async (t: TestContext) => {
  await sql`DELETE FROM org_contact WHERE org_id IN (${testOrgId}, 900002)`;
  await sql`DELETE FROM delivery_log WHERE org_id IN (${testOrgId}, 900002)`;
  await sql`DELETE FROM inbox WHERE message_id LIKE 'test-msg-%'`;
  await sql`INSERT INTO org_contact (org_id, phone, channel) VALUES (${testOrgId}, '+380501234567', 'whatsapp')`;

  await t.test("valid message is delivered exactly once via FakeChannel", async () => {
    const channel = new FakeChannel();
    const payload = new TextEncoder().encode(JSON.stringify(envelope()));

    const first = await processNotifySend(payload, "test-msg-1", channel);
    assert.equal(first.kind, "processed");
    assert.equal(channel.sent.length, 1);
    assert.equal(channel.sent[0]?.destination, "+380501234567");

    const [logRow] = await sql`SELECT phone_masked, status FROM delivery_log WHERE org_id = ${testOrgId}`;
    assert.equal(logRow?.phone_masked, "+380*****4567");
    assert.equal(logRow?.status, "delivered");

    // Повторна доставка ТОГО САМОГО message_id (симуляція "щонайменше один раз" від брокера) --
    // НЕ надсилає вдруге.
    const second = await processNotifySend(payload, "test-msg-1", channel);
    assert.equal(second.kind, "duplicate");
    assert.equal(channel.sent.length, 1, "дублікат не мав повторно надіслати повідомлення");
  });

  await t.test("invalid payload is rejected, not sent, not retried", async () => {
    const channel = new FakeChannel();
    const badPayload = new TextEncoder().encode(JSON.stringify({ not: "a valid envelope" }));

    const result = await processNotifySend(badPayload, "test-msg-invalid", channel);
    assert.equal(result.kind, "invalid");
    assert.equal(channel.sent.length, 0);
  });

  await t.test("org with no active contacts is suppressed, not failed", async () => {
    const channel = new FakeChannel();
    const payload = new TextEncoder().encode(
      JSON.stringify(envelope({ recipient_org_id: 900_002, dedupe_key: "no-contacts" })),
    );

    const result = await processNotifySend(payload, "test-msg-no-contacts", channel);
    assert.equal(result.kind, "processed");
    if (result.kind === "processed") assert.equal(result.status, "suppressed");
  });

  await t.test("failed delivery does NOT mark inbox — allows retry", async () => {
    const channel = new FakeChannel();
    channel.send = async (dest, text) => {
      channel.sent.push({ destination: dest, text });
      return { status: "failed" as const, error: "group not found" };
    };
    const payload = new TextEncoder().encode(
      JSON.stringify(envelope({ dedupe_key: "fail-retry-test" })),
    );

    const first = await processNotifySend(payload, "test-msg-fail", channel);
    assert.equal(first.kind, "processed");
    if (first.kind === "processed") assert.equal(first.status, "failed");

    const [row] = await sql`SELECT 1 FROM inbox WHERE message_id = 'test-msg-fail'`;
    assert.equal(row, undefined, "failed delivery must NOT be in inbox");

    const retry = await processNotifySend(payload, "test-msg-fail", channel);
    assert.equal(retry.kind, "processed", "retry processes again (not duplicate)");
  });

  await sql.end();
});
