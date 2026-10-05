// Трейт-подібний інтерфейс `Channel` (09 §3.7: "WhatsApp — перша реалізація; далі Signal/e-mail/
// SMS"). `FakeChannel` замінює реальний WhatsApp-клієнт в автотестах (§6) -- жодного реального
// мережевого виклику в CI.

import { maskPhone } from "../mask.ts";

export interface SendResult {
  status: "delivered" | "failed";
  error?: string;
}

export interface Channel {
  send(destination: string, text: string): Promise<SendResult>;
  listGroups?(): Promise<{ id: string; name: string }[]>;
  isReady(): boolean;
}

/** Мінімальна підмножина `whatsapp-web.js`'s `Client`, яку реально використовує канал --
 *  вузький інтерфейс замість імпорту важкого типу з бібліотеки в сигнатуру. `info` --
 *  `whatsapp-web.js` сам виставляє його лише ПІСЛЯ події "ready" (`Client.js`: `this.info = new
 *  ClientInfo(...)` усередині `ready`-гілки) -- найнадійніша ознака "клієнт готовий слати", не
 *  власний прапорець, який довелось би синхронізувати вручну. */
export interface WhatsAppLikeClient {
  readonly info: unknown;
  sendMessage(chatId: string, text: string): Promise<unknown>;
  getChats(): Promise<{ id: { _serialized: string }; name: string; isGroup: boolean }[]>;
  readonly pupPage?: { evaluate: (fn: (...args: unknown[]) => unknown, ...args: unknown[]) => Promise<unknown> };
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

  isReady(): boolean {
    return !!this.client.info;
  }

  async send(destination: string, text: string): Promise<SendResult> {
    if (!this.client.info) {
      throw new ChannelNotReadyError();
    }
    let chatId: string;
    let isGroup = false;
    if (destination.endsWith("@g.us")) {
      chatId = destination;
      isGroup = true;
    } else if (destination.endsWith("@c.us") || /^\d+$/.test(destination.replace(/\D/g, "")) && destination.replace(/\D/g, "").length >= 10) {
      chatId = `${destination.replace(/\D/g, "")}@c.us`;
    } else {
      // Name-based group resolution: look up the group by name
      isGroup = true;
      const resolvedId = await this.resolveGroupChatId(destination);
      if (!resolvedId) {
        return { status: "failed", error: `group "${destination}" not found in bot's chats` };
      }
      chatId = resolvedId;
    }
    try {
      await this.client.sendMessage(chatId, text);
      return { status: "delivered" };
    } catch (e) {
      if (isGroup) {
        return { status: "failed", error: `group send failed: ${destination}` };
      }
      const digitsOnly = destination.replace(/\D/g, "");
      const masked = maskPhone(destination);
      const rawMessage = String(e);
      const scrubbed = rawMessage.split(digitsOnly).join(masked).split(destination).join(masked);
      return { status: "failed", error: scrubbed };
    }
  }

  async listGroups(): Promise<{ id: string; name: string }[]> {
    if (!this.client.info) {
      throw new ChannelNotReadyError();
    }
    return this.getGroupsFallback();
  }

  private async getGroupsFallback(): Promise<{ id: string; name: string }[]> {
    try {
      const chats = await this.client.getChats();
      return chats.filter((c) => c.isGroup).map((c) => ({ id: c.id._serialized, name: c.name }));
    } catch {
      if (!this.client.pupPage) {
        throw new Error("getChats() failed and no pupPage available for fallback");
      }
      // WAWebCollections.Chat exists (sendMessage uses .get()), but .getModelsArray() may be broken.
      // Access .models (Backbone-style) or ._models or iterate _index as fallback.
      const groups = (await this.client.pupPage.evaluate(() => {
        try {
          const Chat = ((window as unknown) as { require: (m: string) => Record<string, unknown> }).require("WAWebCollections").Chat as Record<string, unknown>;
          const models = (typeof Chat.getModelsArray === "function" ? Chat.getModelsArray() : Chat._models ?? Chat.models ?? []) as unknown[];
          const arr = Array.isArray(models) ? models : Array.from(models);
          return arr
            .filter((m: unknown) => (m as { id: { server: string } }).id?.server === "g.us")
            .map((m: unknown) => {
              const chat = m as { id: { _serialized: string }; name?: string; formattedTitle?: string; contact?: { name?: string } };
              return { id: chat.id._serialized, name: chat.name || chat.formattedTitle || chat.contact?.name || "" };
            });
        } catch (e) {
          return [{ id: "__error__", name: String(e) }];
        }
      })) as { id: string; name: string }[];
      if (groups.length === 1 && groups[0]?.id === "__error__") {
        console.error("notifier: getGroupsFallback() Store error:", groups[0]?.name);
        return [];
      }
      return groups;
    }
  }

  private async resolveGroupChatId(name: string): Promise<string | null> {
    try {
      const groups = await this.getGroupsFallback();
      const match = groups.find((g) => g.name === name);
      return match?.id ?? null;
    } catch (e) {
      const errDetail = e instanceof Error ? `${e.name}: ${e.message}` : String(e);
      console.error(`notifier: resolveGroupChatId("${name}") failed:`, errDetail);
      return null;
    }
  }
}

export interface FakeSentMessage {
  destination: string;
  text: string;
}

export class FakeChannel implements Channel {
  public sent: FakeSentMessage[] = [];
  public fakeGroups: { id: string; name: string }[] = [];
  public ready = true;

  isReady(): boolean {
    return this.ready;
  }

  async send(destination: string, text: string): Promise<SendResult> {
    this.sent.push({ destination, text });
    return { status: "delivered" };
  }

  async listGroups(): Promise<{ id: string; name: string }[]> {
    return this.fakeGroups;
  }
}
