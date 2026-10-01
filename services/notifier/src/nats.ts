// Транспорт (09-messaging.md §3.7, §4): @nats-io/transport-node + @nats-io/jetstream (nats.js
// v3) + @nats-io/kv (стан прив'язки WhatsApp, §4 цього файлу нижче в CLAUDE.md).

import { connect, type NatsConnection } from "@nats-io/transport-node";
import {
  AckPolicy,
  jetstream,
  jetstreamManager,
  type JetStreamClient,
  type JetStreamManager,
} from "@nats-io/jetstream";
import { Kvm, type KV } from "@nats-io/kv";
import { config } from "./config.ts";

export interface NatsHandles {
  nc: NatsConnection;
  js: JetStreamClient;
  jsm: JetStreamManager;
  kv: KV;
}

const NOTIFY_CMD_STREAM = "NOTIFY_CMD";
const NOTIFY_CMD_SUBJECT = "vyshkil.notify.send.v1";
const NOTIFY_RESULT_SUBJECT = "vyshkil.notify.result.v1";
const STATUS_BUCKET = "notifier_status";
const DURABLE_CONSUMER = "notifier-whatsapp";
export const MAX_DELIVER = 5;

/** З'єднання + KV bucket для стану прив'язки (§4: TTL 60с, history 1 -- лише ОСТАННІЙ стан
 *  має значення, попередні не потрібні). Стрім `NOTIFY_CMD` МАЄ вже існувати (створює його
 *  relay/адмін-крок на боці `app`, не тут -- `bus`/`server` у Rust-частині володіють схемою
 *  стрімів, notifier лише читає). */
/** `@nats-io/nats-core`'s `servers` option не приймає `user:pass@host:port` як єдиний рядок --
 *  його внутрішній парсер рахує двокрапки, щоб відрізнити IPv6, і рядок із вбудованими
 *  обліковими даними (дві двокрапки: user:pass і host:port) хибно розпізнається як IPv6,
 *  після чого обгортається в "[...]" і падає на `new URL()` (`ERR_INVALID_URL`). Реальна
 *  грабля, зловлена на dev-VM при першому `docker compose up` з увімкненою автентифікацією
 *  NATS (Фаза 3) -- розбираємо URL самі, обліковки йдуть окремими полями `user`/`pass`. */
function parseNatsUrl(raw: string): { servers: string; user?: string; pass?: string } {
  const u = new URL(raw);
  return {
    servers: `${u.hostname}:${u.port}`,
    user: u.username ? decodeURIComponent(u.username) : undefined,
    pass: u.password ? decodeURIComponent(u.password) : undefined,
  };
}

export async function connectNats(): Promise<NatsHandles> {
  const nc = await connect(parseNatsUrl(config.natsUrl));
  const jsm = await jetstreamManager(nc);
  const js = jetstream(nc);
  const kvm = new Kvm(nc);
  const kv = await kvm.create(STATUS_BUCKET, { history: 1, ttl: 60_000 });

  // Ідемпотентно (get-or-create, той самий підхід, що `bus::ensure_stream` на боці Rust) --
  // notifier володіє схемою ЦИХ двох стрімів (§3.3-таблиця: NOTIFY_CMD work-queue, NOTIFY_RESULT
  // limits), не app/relay. `info()` спершу (а не голий `add()` + catch на текст помилки рядком,
  // який ламкий до версії клієнта) -- "нема такого стріму" і "стрім є, конфліктна конфігурація"
  // це РІЗНІ ситуації, перша очікувана на кожному старті, друга мала б впасти.
  await ensureStream(jsm, NOTIFY_CMD_STREAM, [NOTIFY_CMD_SUBJECT]);
  await ensureStream(jsm, "NOTIFY_RESULT", [NOTIFY_RESULT_SUBJECT]);
  await ensureStream(jsm, "DLQ", ["vyshkil.dlq.>"]);

  return { nc, js, jsm, kv };
}

async function ensureStream(jsm: JetStreamManager, name: string, subjects: string[]): Promise<void> {
  try {
    await jsm.streams.info(name);
  } catch {
    await jsm.streams.add({ name, subjects });
  }
}

export async function bindNotifyCmdConsumer({ js, jsm }: NatsHandles) {
  await jsm.consumers.add(NOTIFY_CMD_STREAM, {
    durable_name: DURABLE_CONSUMER,
    ack_policy: AckPolicy.Explicit,
    ack_wait: 30_000_000_000, // 30с у наносекундах (JetStream consumer config -- nanos, не мс)
    max_deliver: MAX_DELIVER,
    filter_subject: NOTIFY_CMD_SUBJECT,
  });
  return js.consumers.get(NOTIFY_CMD_STREAM, DURABLE_CONSUMER);
}

export async function publishNotifyResult(
  { js }: NatsHandles,
  envelope: unknown,
  msgId: string,
): Promise<void> {
  await js.publish(NOTIFY_RESULT_SUBJECT, new TextEncoder().encode(JSON.stringify(envelope)), {
    msgID: msgId,
  });
}

export async function publishStatus(
  { kv }: NatsHandles,
  status: { state: string; qr?: string; pairing_code?: string; phone_masked?: string },
): Promise<void> {
  await kv.put(
    "whatsapp",
    new TextEncoder().encode(JSON.stringify({ ...status, updated_at: new Date().toISOString() })),
  );
}
