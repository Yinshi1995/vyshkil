// Машина станів прив'язки WhatsApp-сесії (09-messaging.md §4): starting → pairing (qr|code) →
// connected → needs_pairing → pairing … . QR/pairing-код НІКОЛИ не пишуться в логи/БД/JetStream
// -- лише в NATS KV (TTL 60с, `publishStatus`) і в пам'яті цього процесу.

import type { Client } from "whatsapp-web.js";
import { publishStatus, type NatsHandles } from "./nats.ts";
import { config } from "./config.ts";
import { maskPhone } from "./mask.ts";
import { purgeSession } from "./whatsapp/client.ts";

export type PairingState = "starting" | "pairing" | "connected" | "needs_pairing";

export class PairingStateMachine {
  private client: Client;
  private nats: NatsHandles;
  private state: PairingState = "starting";
  private phoneMasked: string | undefined;
  private lastQr: string | undefined;
  private lastPairingCode: string | undefined;
  private firstEventFired = false;
  /** Викликається на `needs_pairing` (сесію відкликано) -- застосунок показує дзвіночок,
   *  НЕ через WhatsApp (§3: "сповіщення адміна в застосунку"). Реалізація публікації в цей
   *  внутрішній канал -- поза обсягом notifier'а, викликач (index.ts) підключає її ззовні. */
  public onNeedsPairing: () => void = () => {};
  /** Startup watchdog: викликається при першому значущому івенті (qr/ready/auth_failure).
   *  index.ts скидає watchdog-таймер через цей колбек. */
  public onFirstEvent: () => void = () => {};

  constructor(client: Client, nats: NatsHandles) {
    this.client = client;
    this.nats = nats;
    this.wireEvents();
  }

  private fireFirstEvent(): void {
    if (!this.firstEventFired) {
      this.firstEventFired = true;
      this.onFirstEvent();
    }
  }

  private wireEvents(): void {
    this.client.on("qr", async (qr: string) => {
      this.fireFirstEvent();
      this.state = "pairing";
      this.lastQr = qr;
      this.lastPairingCode = undefined;
      await publishStatus(this.nats, { state: this.state, qr });
      if (config.waPrintQrToLogs) {
        const qrcodeTerminal = await import("qrcode-terminal");
        qrcodeTerminal.default.generate(qr, { small: true });
      }
    });

    this.client.on("authenticated", async () => {
      this.fireFirstEvent();
      this.lastQr = undefined;
      this.lastPairingCode = undefined;
    });

    this.client.on("ready", async () => {
      this.fireFirstEvent();
      this.state = "connected";
      this.lastQr = undefined;
      this.lastPairingCode = undefined;
      const info = this.client.info;
      this.phoneMasked = info?.wid?.user ? maskPhone(info.wid.user) : undefined;
      await publishStatus(this.nats, { state: this.state, phone_masked: this.phoneMasked });
    });

    this.client.on("auth_failure", async (msg: string) => {
      this.fireFirstEvent();
      this.state = "needs_pairing";
      this.lastQr = undefined;
      this.lastPairingCode = undefined;
      console.error("WhatsApp auth_failure:", msg, "— видаляю зіпсовану сесію");
      purgeSession();
      await publishStatus(this.nats, { state: this.state });
      this.onNeedsPairing();
    });

    this.client.on("disconnected", async (reason: string) => {
      // LOGOUT (сесію відкликано з телефону) -- needs_pairing. Інші причини (мережа тощо) --
      // сам whatsapp-web.js перепідключається внутрішньо; ми лише відображаємо needs_pairing
      // для LOGOUT конкретно (§3: "disconnected → перепідключення з backoff" стосується
      // мережевих причин, LOGOUT — інше).
      if (reason === "LOGOUT" || reason === "NAVIGATION") {
        this.state = "needs_pairing";
        this.lastQr = undefined;
        this.lastPairingCode = undefined;
        purgeSession();
        await publishStatus(this.nats, { state: this.state });
        this.onNeedsPairing();
      }
    });
  }

  getState(): PairingState {
    return this.state;
  }

  getStatusSnapshot(): { state: string; qr?: string; pairing_code?: string; phone_masked?: string } {
    return {
      state: this.state,
      qr: this.lastQr,
      pairing_code: this.lastPairingCode,
      phone_masked: this.phoneMasked,
    };
  }

  async requestPairingCode(phoneNumber: string): Promise<string> {
    this.state = "pairing";
    const code = await this.client.requestPairingCode(phoneNumber);
    this.lastPairingCode = code;
    this.lastQr = undefined;
    await publishStatus(this.nats, { state: this.state, pairing_code: code });
    return code;
  }

  async logout(): Promise<void> {
    try {
      await this.client.logout();
    } catch (e) {
      console.warn("notifier: client.logout() помилка (ігноруємо):", e);
    }
    this.state = "needs_pairing";
    this.lastQr = undefined;
    this.lastPairingCode = undefined;
    purgeSession();
    await publishStatus(this.nats, { state: this.state });
    // Перезапустити процес — Docker restart policy піднімає з чистою сесією і QR
    console.log("notifier: сесію видалено після logout, перезапуск для нового QR");
    setTimeout(() => process.exit(0), 500);
  }
}
