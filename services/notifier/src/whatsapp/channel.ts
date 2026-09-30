// Трейт-подібний інтерфейс `Channel` (09 §3.7: "WhatsApp — перша реалізація; далі Signal/e-mail/
// SMS"). `FakeChannel` замінює реальний WhatsApp-клієнт в автотестах (§6) -- жодного реального
// мережевого виклику в CI.

export interface SendResult {
  status: "delivered" | "failed";
  error?: string;
}

export interface Channel {
  send(phone: string, text: string): Promise<SendResult>;
}

/** Мінімальна підмножина `whatsapp-web.js`'s `Client`, яку реально використовує канал --
 *  вузький інтерфейс замість імпорту важкого типу з бібліотеки в сигнатуру. */
export interface WhatsAppLikeClient {
  sendMessage(chatId: string, text: string): Promise<unknown>;
}

export class WhatsAppChannel implements Channel {
  private client: WhatsAppLikeClient;

  constructor(client: WhatsAppLikeClient) {
    this.client = client;
  }

  async send(phone: string, text: string): Promise<SendResult> {
    const chatId = `${phone.replace(/\D/g, "")}@c.us`;
    try {
      await this.client.sendMessage(chatId, text);
      return { status: "delivered" };
    } catch (e) {
      return { status: "failed", error: String(e) };
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
