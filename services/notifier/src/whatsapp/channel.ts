// Трейт-подібний інтерфейс `Channel` (09 §3.7: "WhatsApp — перша реалізація; далі Signal/e-mail/
// SMS"). `FakeChannel` замінює реальний WhatsApp-клієнт в автотестах (§6) -- жодного реального
// мережевого виклику в CI.

import { maskPhone } from "../mask.ts";

export interface SendResult {
  status: "delivered" | "failed";
  error?: string;
}

export interface Channel {
  send(phone: string, text: string): Promise<SendResult>;
}

/** Мінімальна підмножина `whatsapp-web.js`'s `Client`, яку реально використовує канал --
 *  вузький інтерфейс замість імпорту важкого типу з бібліотеки в сигнатуру. `info` --
 *  `whatsapp-web.js` сам виставляє його лише ПІСЛЯ події "ready" (`Client.js`: `this.info = new
 *  ClientInfo(...)` усередині `ready`-гілки) -- найнадійніша ознака "клієнт готовий слати", не
 *  власний прапорець, який довелось би синхронізувати вручну. */
export interface WhatsAppLikeClient {
  readonly info: unknown;
  sendMessage(chatId: string, text: string): Promise<unknown>;
}

/** Відрізняється від звичайної невдачі відправки (ту `send()` ловить і повертає як
 *  `{status:'failed'}`, термінальний стан): "клієнт ще не прив'язаний" МАЄ призвести до retry
 *  (nak), не до негайного "failed" — тому кидається, не повертається (09 §3: "команди в
 *  черзі... чекають (nak із затримкою), а не падають у DLQ"). */
export class ChannelNotReadyError extends Error {
  constructor() {
    super("WhatsApp-клієнт ще не прив'язаний (needs_pairing)");
    this.name = "ChannelNotReadyError";
  }
}

export class WhatsAppChannel implements Channel {
  private client: WhatsAppLikeClient;

  constructor(client: WhatsAppLikeClient) {
    this.client = client;
  }

  async send(phone: string, text: string): Promise<SendResult> {
    if (!this.client.info) {
      throw new ChannelNotReadyError();
    }
    const digitsOnly = phone.replace(/\D/g, "");
    const chatId = `${digitsOnly}@c.us`;
    try {
      await this.client.sendMessage(chatId, text);
      return { status: "delivered" };
    } catch (e) {
      // §5: "номери в логах маскуються" -- стосується й ТЕКСТУ помилки, не лише delivery_log's
      // `phone_masked`-стовпця: `whatsapp-web.js`'s власні помилки часто вставляють `chatId`
      // (сам номер) просто в текст (напр. "could not send to 380501234567@c.us"), і цей `error`
      // рядок іде і в журнал (delivery_log.error), і в `console.error` в index.ts -- обидва
      // шляхи мали б витекти повний номер без цього маскування.
      const masked = maskPhone(phone);
      const rawMessage = String(e);
      const scrubbed = rawMessage.split(digitsOnly).join(masked).split(phone).join(masked);
      return { status: "failed", error: scrubbed };
    }
  }
}

export interface FakeSentMessage {
  phone: string;
  text: string;
}

export class FakeChannel implements Channel {
  public sent: FakeSentMessage[] = [];

  async send(phone: string, text: string): Promise<SendResult> {
    this.sent.push({ phone, text });
    return { status: "delivered" };
  }
}
