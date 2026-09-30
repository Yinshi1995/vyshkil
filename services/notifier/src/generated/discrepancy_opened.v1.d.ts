export type DiscrepancyMetric =
  "planned_count" | "arrived_count" | "in_training_count" | "planned_start" | "planned_end" | "site_id";

export interface EnvelopeDiscrepancyOpenedV1 {
  causation_id?: string | null;
  correlation_id: string;
  id: string;
  occurred_at: string;
  payload: DiscrepancyOpened;
  producer: string;
  type: string;
  version: number;
  [k: string]: unknown;
}
export interface DiscrepancyOpened {
  discrepancy_id: number;
  group_id?: number | null;
  metric: DiscrepancyMetric;
  org_id: number;
  [k: string]: unknown;
}
