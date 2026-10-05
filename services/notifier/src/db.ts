// `postgres` (porsager), без ORM -- 3 таблиці, прямий SQL (09 §3.7).

import postgres from "postgres";
import { config } from "./config.ts";

export const sql = postgres(config.databaseUrl);

/** Ідемпотентність (09 §3.2) -- перевірка й позначення в ОДНІЙ транзакції з побічним ефектом
 *  обробки, викликачем (`whatsapp/channel.ts`'s send-плюс-inbox-запис). */
export async function isAlreadyProcessed(messageId: string): Promise<boolean> {
  const rows = await sql`SELECT 1 FROM inbox WHERE message_id = ${messageId}`;
  return rows.length > 0;
}

export async function markProcessed(tx: postgres.TransactionSql, messageId: string): Promise<void> {
  await tx`INSERT INTO inbox (message_id) VALUES (${messageId}) ON CONFLICT DO NOTHING`;
}

export interface OrgContact {
  phone: string;
  channel: string;
  kind: string;
}

export async function contactsForOrg(orgId: number): Promise<OrgContact[]> {
  const rows = await sql<OrgContact[]>`
    SELECT phone, channel, kind FROM org_contact WHERE org_id = ${orgId} AND active
  `;
  return rows;
}

export async function upsertContact(orgId: number, phone: string, kind: string, active: boolean): Promise<void> {
  await sql`
    INSERT INTO org_contact (org_id, phone, kind, active)
    VALUES (${orgId}, ${phone}, ${kind}, ${active})
    ON CONFLICT (org_id, phone) DO UPDATE SET kind = ${kind}, active = ${active}
  `;
}

export async function removeContact(orgId: number, phone: string): Promise<void> {
  await sql`DELETE FROM org_contact WHERE org_id = ${orgId} AND phone = ${phone}`;
}

export interface SyncContact {
  org_id: number;
  phone: string;
  kind: string;
  active: boolean;
}

export async function replaceAllContacts(contacts: SyncContact[]): Promise<void> {
  await sql.begin(async (tx) => {
    await tx`DELETE FROM org_contact`;
    for (const c of contacts) {
      await tx`INSERT INTO org_contact (org_id, phone, kind, active)
        VALUES (${c.org_id}, ${c.phone}, ${c.kind}, ${c.active})`;
    }
  });
}

/** `phone_masked` -- викликач МАЄ передати вже замаскований рядок (§5: "номери в логах
 *  маскуються"), ця функція сама не маскує -- маскування живе поруч із номером у
 *  `whatsapp/channel.ts`, не тут. */
export async function insertDeliveryLog(
  tx: postgres.TransactionSql | postgres.Sql,
  entry: { orgId: number; phoneMasked: string; template: string; status: string; error?: string },
): Promise<void> {
  await tx`
    INSERT INTO delivery_log (org_id, phone_masked, template, status, error)
    VALUES (${entry.orgId}, ${entry.phoneMasked}, ${entry.template}, ${entry.status}, ${entry.error ?? null})
  `;
}
