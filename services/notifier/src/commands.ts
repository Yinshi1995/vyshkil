// Команди від адмінки (09-messaging.md §4) -- NATS request-reply (core, НЕ JetStream: разові
// команди, не потребують персистентності/повторів). Права на ці subject-и обмежені обліковим
// записам `app`/`notifier` на боці NATS-конфігу (Фаза 3).

import type { NatsConnection, Msg } from "@nats-io/transport-node";
import { z } from "zod";
import type { PairingStateMachine } from "./pairing.ts";
import type { Channel } from "./whatsapp/channel.ts";
import { contactsForOrg, upsertContact, removeContact, replaceAllContacts } from "./db.ts";
import { renderTemplate } from "./templates.ts";

const SUBJECT_PAIR_QR = "vyshkil.notifier.whatsapp.pair.qr";
const SUBJECT_PAIR_CODE = "vyshkil.notifier.whatsapp.pair.code";
const SUBJECT_LOGOUT = "vyshkil.notifier.whatsapp.logout";
const SUBJECT_TEST = "vyshkil.notifier.whatsapp.test";
const SUBJECT_GROUPS_LIST = "vyshkil.notifier.whatsapp.groups.list";
const SUBJECT_CONTACTS_UPSERT = "vyshkil.notifier.whatsapp.contacts.upsert";
const SUBJECT_CONTACTS_REMOVE = "vyshkil.notifier.whatsapp.contacts.remove";
const SUBJECT_CONTACTS_SYNC = "vyshkil.notifier.whatsapp.contacts.sync";

const pairCodeRequestSchema = z.object({ phone: z.string().min(5) });
const testRequestSchema = z.object({ org_id: z.number().int() });
const contactUpsertSchema = z.object({
  org_id: z.number().int(),
  phone: z.string().min(1),
  kind: z.enum(["personal", "bot", "group"]),
  active: z.boolean(),
});
const contactRemoveSchema = z.object({
  org_id: z.number().int(),
  phone: z.string().min(1),
});
const contactSyncSchema = z.object({
  contacts: z.array(z.object({
    org_id: z.number().int(),
    phone: z.string().min(1),
    kind: z.enum(["personal", "bot", "group"]),
    active: z.boolean(),
  })),
});

function reply(msg: Msg, body: unknown): void {
  msg.respond(new TextEncoder().encode(JSON.stringify(body)));
}

export function wireAdminCommands(
  nc: NatsConnection,
  pairing: PairingStateMachine,
  channel: Channel,
): void {
  const qrSub = nc.subscribe(SUBJECT_PAIR_QR);
  (async () => {
    for await (const msg of qrSub) {
      // QR публікується автоматично клієнтом на подію "qr" (pairing.ts) -- цей запит лише
      // підтверджує поточний стан у відповіді, не генерує новий QR за вимогою (те саме QR-
      // оновлення й так триває кожні ~20с, поки клієнт непідключений).
      reply(msg, { state: pairing.getState() });
    }
  })();

  const codeSub = nc.subscribe(SUBJECT_PAIR_CODE);
  (async () => {
    for await (const msg of codeSub) {
      try {
        const { phone } = pairCodeRequestSchema.parse(JSON.parse(msg.string()));
        const code = await pairing.requestPairingCode(phone);
        reply(msg, { pairing_code: code });
      } catch (e) {
        reply(msg, { error: String(e) });
      }
    }
  })();

  const logoutSub = nc.subscribe(SUBJECT_LOGOUT);
  (async () => {
    for await (const msg of logoutSub) {
      await pairing.logout();
      reply(msg, { ok: true });
    }
  })();

  const testSub = nc.subscribe(SUBJECT_TEST);
  (async () => {
    for await (const msg of testSub) {
      try {
        const { org_id: orgId } = testRequestSchema.parse(JSON.parse(msg.string()));
        const contacts = await contactsForOrg(orgId);
        const text = renderTemplate("discrepancy_detected");
        const results = await Promise.all(contacts.map((c) => channel.send(c.phone, text)));
        reply(msg, { sent: results.length, results });
      } catch (e) {
        reply(msg, { error: String(e) });
      }
    }
  })();

  const groupsListSub = nc.subscribe(SUBJECT_GROUPS_LIST);
  (async () => {
    for await (const msg of groupsListSub) {
      try {
        if (!channel.listGroups) {
          reply(msg, { groups: [], error: "listGroups not supported" });
          continue;
        }
        const groups = await channel.listGroups();
        reply(msg, { groups });
      } catch (e) {
        reply(msg, { error: String(e) });
      }
    }
  })();

  const contactsUpsertSub = nc.subscribe(SUBJECT_CONTACTS_UPSERT);
  (async () => {
    for await (const msg of contactsUpsertSub) {
      try {
        const data = contactUpsertSchema.parse(JSON.parse(msg.string()));
        await upsertContact(data.org_id, data.phone, data.kind, data.active);
        reply(msg, { ok: true });
      } catch (e) {
        reply(msg, { error: String(e) });
      }
    }
  })();

  const contactsRemoveSub = nc.subscribe(SUBJECT_CONTACTS_REMOVE);
  (async () => {
    for await (const msg of contactsRemoveSub) {
      try {
        const data = contactRemoveSchema.parse(JSON.parse(msg.string()));
        await removeContact(data.org_id, data.phone);
        reply(msg, { ok: true });
      } catch (e) {
        reply(msg, { error: String(e) });
      }
    }
  })();

  const contactsSyncSub = nc.subscribe(SUBJECT_CONTACTS_SYNC);
  (async () => {
    for await (const msg of contactsSyncSub) {
      try {
        const data = contactSyncSchema.parse(JSON.parse(msg.string()));
        await replaceAllContacts(data.contacts);
        reply(msg, { ok: true, count: data.contacts.length });
      } catch (e) {
        reply(msg, { error: String(e) });
      }
    }
  })();
}
