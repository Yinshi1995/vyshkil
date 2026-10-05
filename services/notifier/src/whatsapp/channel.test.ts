import { strict as assert } from "node:assert";
import { test } from "node:test";
import { ChannelNotReadyError, WhatsAppChannel } from "./channel.ts";

// 09 §3: "команди в черзі... чекають (nak), а не падають у DLQ" під час needs_pairing -- це
// МАЄ бути кинута помилка (retry-able), не звичайний {status:'failed'} (термінальний стан).
// Реальна прогалина, знайдена при написанні §6-сценаріїв: оригінальна реалізація ловила ВСІ
// помилки `sendMessage` однаково, тож needs_pairing ніколи б фактично не чекав -- одразу
// ack'ався б як "failed".
test("send() throws ChannelNotReadyError (not a failed-status result) when client isn't ready yet", async () => {
  const notReadyClient = { info: undefined, sendMessage: async () => "unreachable", getChats: async () => [] };
  const channel = new WhatsAppChannel(notReadyClient);

  await assert.rejects(() => channel.send("+380501234567", "текст"), ChannelNotReadyError);
});

// Приватність (09 §3.5/§5): помилка від WhatsApp-клієнта часто вставляє сам номер у текст --
// цей тест довбає конкретно ТОЙ шлях (не лише happy-path), де номер міг би витекти через
// `delivery_log.error`/логи.
test("a thrown error containing the raw phone number is scrubbed before being returned", async () => {
  const phone = "+380501234567";
  const fakeClient = {
    info: { wid: { user: "380999999999" } },
    sendMessage: async (chatId: string) => {
      throw new Error(`could not send message to ${chatId}: timeout`);
    },
    getChats: async () => [],
  };
  const channel = new WhatsAppChannel(fakeClient);

  const result = await channel.send(phone, "текст");

  assert.equal(result.status, "failed");
  assert.ok(result.error, "помилка має бути присутньою");
  assert.ok(!result.error!.includes("380501234567"), `повний номер просочився в: ${result.error}`);
  assert.ok(result.error!.includes("+380*****4567"), `маскований номер мав лишитись видимим для діагностики: ${result.error}`);
});
