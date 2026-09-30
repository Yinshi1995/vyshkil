// Інтеграційний тест (09-messaging.md §6): реальний NATS, реальна публікація в DLQ -- не мок.
// Потребує TEST_NATS_URL (без нього -- пропущено).

import { strict as assert } from "node:assert";
import { test } from "node:test";
import { connect } from "@nats-io/transport-node";
import { jetstream, jetstreamManager } from "@nats-io/jetstream";
import { publishToDlq, DLQ_SUBJECT } from "./dlq.ts";

const hasNats = !!process.env.TEST_NATS_URL;
if (!hasNats) {
  console.error("TEST_NATS_URL не задано — dlq.test.ts пропущено");
}

test("publishToDlq writes a retrievable entry with reason and original payload", { skip: !hasNats }, async () => {
  const url = process.env.TEST_NATS_URL!;
  const nc = await connect({ servers: url });
  const js = jetstream(nc);
  const jsm = await jetstreamManager(nc);

  const streamName = "TEST_DLQ_" + Date.now();
  await jsm.streams.add({ name: streamName, subjects: [DLQ_SUBJECT] });

  const originalPayload = new TextEncoder().encode(JSON.stringify({ hello: "world" }));
  await publishToDlq({ js } as never, "vyshkil.notify.send.v1", originalPayload, "zod: recipient_org_id missing");

  const consumer = await jsm.consumers.add(streamName, {
    durable_name: "test-dlq-consumer",
    filter_subject: DLQ_SUBJECT,
  });
  const bound = await js.consumers.get(streamName, "test-dlq-consumer");
  const msgs = await bound.fetch({ max_messages: 1, expires: 5_000 });
  let received: unknown;
  for await (const m of msgs) {
    received = JSON.parse(m.string());
    m.ack();
  }

  assert.ok(received, "DLQ мало отримати запис");
  const entry = received as { original_subject: string; reason: string; payload_base64: string };
  assert.equal(entry.original_subject, "vyshkil.notify.send.v1");
  assert.match(entry.reason, /recipient_org_id/);
  const decoded = Buffer.from(entry.payload_base64, "base64").toString("utf-8");
  assert.equal(decoded, new TextDecoder().decode(originalPayload));

  await jsm.streams.delete(streamName);
  await nc.close();
});
