export type NotifyStatus = "delivered" | "failed" | "suppressed";

export interface EnvelopeNotifyResultV1 {
  causation_id?: string | null;
  correlation_id: string;
  id: string;
  occurred_at: string;
  payload: NotifyResult;
  producer: string;
  type: string;
  version: number;
  [k: string]: unknown;
}
export interface NotifyResult {
  dedupe_key: string;
  recipient_org_id: number;
  status: NotifyStatus;
  [k: string]: unknown;
}
