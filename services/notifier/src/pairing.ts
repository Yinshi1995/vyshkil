// Машина станів прив'язки WhatsApp-сесії (09-messaging.md §4): starting → pairing (qr|code) →
// connected → needs_pairing → pairing … . QR/pairing-код НІКОЛИ не пишуться в логи/БД/JetStream
// -- лише в NATS KV (TTL 60с, `publishStatus`) і в пам'яті цього процесу.

import type { Client } from "whatsapp-web.js";
import { publishStatus, type NatsHandles } from "./nats.ts";
import { config } from "./config.ts";
import { maskPhone } from "./mask.ts";

export type PairingState = "starting" | "pairing" | "connected" | "needs_pairing";

export class PairingStateMachine {
  private client: Client;
  private nats: NatsHandles;
  private state: PairingState = "starting";
  private phoneMasked: string | undefined;
  /** Викликається на `needs_pairing` (сесію відкликано) -- застосунок показує дзвіночок,
   *  НЕ через WhatsApp (§3: "сповіщення адміна в застосунку"). Реалізація публікації в цей
   *  внутрішній канал -- поза обсягом notifier'а, викликач (index.ts) підключає її ззовні. */
  public onNeedsPairing: () => void = () => {};

  constructor(client: Client, nats: NatsHandles) {
    this.client = client;
    this.nats = nats;
    this.wireEvents();
  }

  private wireEvents(): void {
    this.client.on("qr", async (qr: string) => {
      this.state = "pairing";
      await publishStatus(this.nats, { state: this.state, qr });
      if (config.waPrintQrToLogs) {
        // Фолбек, коли адмінка недоступна (§4) -- лише за явним прапорцем, вимкнено за
        // замовчуванням (логи можуть збиратися).
        const qrcodeTerminal = await import("qrcode-terminal");
        qrcodeTerminal.default.generate(qr, { small: true });
      }
    });

    this.client.on("authenticated", async () => {
      // "authenticated" ще не "ready" (сесія прийнята, клієнт ще довантажується) -- KV лишається
      // "pairing" до реального "ready", щоб адмінка не показала "підключено" зарано.
    });

    this.client.on("ready", async () => {
      this.state = "connected";
      const info = this.client.info;
      this.phoneMasked = info?.wid?.user ? maskPhone(info.wid.user) : undefined;
      await publishStatus(this.nats, { state: this.state, phone_masked: this.phoneMasked });
    });

    this.client.on("auth_failure", async (msg: string) => {
      this.state = "needs_pairing";
      await publishStatus(this.nats, { state: this.state });
      console.error("WhatsApp auth_failure:", msg);
      this.onNeedsPairing();
    });

    this.client.on("disconnected", async (reason: string) => {
      // LOGOUT (сесію відкликано з телефону) -- needs_pairing. Інші причини (мережа тощо) --
      // сам whatsapp-web.js перепідключається внутрішньо; ми лише відображаємо needs_pairing
      // для LOGOUT конкретно (§3: "disconnected → перепідключення з backoff" стосується
      // мережевих причин, LOGOUT — інше).
      if (reason === "LOGOUT" || reason === "NAVIGATION") {
        this.state = "needs_pairing";
        await publishStatus(this.nats, { state: this.state });
        this.onNeedsPairing();
      }
    });
  }

  getState(): PairingState {
    return this.state;
  }

  async requestPairingCode(phoneNumber: string): Promise<string> {
    this.state = "pairing";
    const code = await this.client.requestPairingCode(phoneNumber);
    await publishStatus(this.nats, { state: this.state, pairing_code: code });
    return code;
  }

  async logout(): Promise<void> {
    await this.client.logout();
    this.state = "needs_pairing";
    await publishStatus(this.nats, { state: this.state });
  }
}
