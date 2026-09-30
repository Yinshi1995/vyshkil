/**
 * Шаблон сповіщення — сам текст (знеособлений, 04 §5) рендерить `notifier` за цим варіантом,
 * не отримує його рядком через брокер.
 */
export type NotifyTemplate = "discrepancy_detected";

export interface EnvelopeNotifySendV1 {
  causation_id?: string | null;
  correlation_id: string;
  id: string;
  occurred_at: string;
  payload: NotifySend;
  producer: string;
  type: string;
  version: number;
  [k: string]: unknown;
}
export interface NotifySend {
  dedupe_key: string;
  recipient_org_id: number;
  template: NotifyTemplate;
  [k: string]: unknown;
}
