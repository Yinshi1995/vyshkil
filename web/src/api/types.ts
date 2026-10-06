export interface Actor {
  org_id: number;
  role: string;
  org_label: string;
}

export interface AuthUser {
  user_id: number;
  actor: Actor | null;
  display_name: string | null;
  callsign: string | null;
  avatar_url: string | null;
  can_see_org_names: boolean;
}

export interface UserRoleRow {
  org_id: number;
  role: string;
  org_label: string;
}

export interface LoginRequest {
  login: string;
  password: string;
}

export interface LoginResponse {
  success: boolean;
  error: string | null;
  display_name: string | null;
  roles: UserRoleRow[];
  must_change_password: boolean;
}

export interface AdminUserRow {
  id: number;
  login: string;
  display_name: string | null;
  is_active: boolean;
  must_change_password: boolean;
  can_see_org_names: boolean;
  roles: UserRoleRow[];
  created_at: string;
}

export interface AdminSubmissionRow {
  id: number;
  org_label: string;
  source_type: string;
  status: string;
  as_of_date: string;
  updated_at: string;
}

export interface AdminGroupRow {
  id: number;
  org_label: string;
  training_kind: string;
  vos_label: string;
  site_label: string;
  planned_start: string;
  planned_end: string;
  planned_count: number;
  arrived_count: number;
  in_training_count: number;
}

export interface OrgDetail {
  id: number;
  short_name: string;
  full_name: string | null;
  kind: string;
  is_active: boolean;
  name_history: [string, string, string | null][];
  status_history: [string, string, string | null, string | null][];
}

export interface OrgChild {
  id: number;
  label: string;
}

export interface KindCount {
  kind: string;
  count: number;
}

export interface RecentSub {
  id: number;
  org_label: string;
  source_type: string;
  updated_at: string;
}

export interface RecentDisc {
  id: number;
  org_label: string;
  metric_label: string;
  status: string;
  created_at: string;
}

export interface DashboardStats {
  total_orgs: number;
  total_groups: number;
  total_submissions: number;
  total_discrepancies: number;
  groups_by_kind: KindCount[];
  recent_submissions: RecentSub[];
  recent_discrepancies: RecentDisc[];
}

export interface DiscrepancyValue {
  submission_id: number;
  source_label: string;
  value: string;
}

export interface DiscrepancyRow {
  id: number;
  kind: string;
  org_id: number;
  org_label: string;
  group_id: number | null;
  group_label: string | null;
  as_of: string;
  metric: string;
  metric_label: string;
  values: DiscrepancyValue[];
  status: string;
  created_at: string;
}

export interface ReportedGroupSnapshot {
  submission_id: number;
  source_label: string;
  training_kind: string;
  vos_label: string | null;
  position_label: string | null;
  course_label: string | null;
  site_label: string;
  organizer_label: string | null;
  planned_start: string;
  planned_end: string;
  equipment_text: string | null;
  basis_doc_number: string | null;
  note: string | null;
  planned_count: number;
  arrived_count: number;
  in_training_count: number;
}

export interface DiscrepancyComparison {
  metric: string;
  metric_label: string;
  rows: ReportedGroupSnapshot[];
}

export interface AccountInfo {
  login: string;
  full_name: string | null;
  first_name: string | null;
  last_name: string | null;
  rank: string | null;
  phone: string | null;
  callsign: string | null;
  delta_nick: string | null;
  avatar_url: string | null;
  roles: string[];
  created_at: string;
}

export interface DirectoryUser {
  user_id: number;
  login: string;
  display_name: string | null;
  first_name: string | null;
  last_name: string | null;
  rank: string | null;
  phone: string | null;
  callsign: string | null;
  delta_nick: string | null;
  avatar_url: string | null;
  is_active: boolean;
  role: string;
  org_label: string;
}

export interface OrgSearchResult {
  org_id: number;
  label: string;
  matched_raw: string;
  is_exact: boolean;
}

export interface OrgHierarchyNode {
  id: number;
  short_name: string;
  kind: string;
  echelon: string | null;
  is_active: boolean;
  parent_id: number | null;
}

export interface OrgNumber {
  number_kind: string | null;
  number: string | null;
}

export interface SubordinationLink {
  id: number;
  other_org_id: number;
  other_org_name: string;
  axis: string;
  valid_from: string;
  valid_to: string | null;
}

export interface WhatsAppStatus {
  state: string;
  qr_svg: string | null;
  pairing_code: string | null;
  phone_masked: string | null;
  updated_at: string;
}

// ---------------------------------------------------------------------------
// WhatsApp routing types
// ---------------------------------------------------------------------------

export interface WaDestination {
  id: number;
  org_id: number;
  kind: string;
  phone_masked: string;
  group_id: string | null;
  is_active: boolean;
  created_at: string;
}

export interface WaGroupInfo {
  id: string;
  name: string;
}

export interface WaSubscription {
  id: number;
  destination_id: number;
  notification_type_code: string;
  type_name: string;
  is_active: boolean;
}

export interface NotificationType {
  code: string;
  name: string;
  scope: string;
}

// ---------------------------------------------------------------------------
// Data Workspace types
// ---------------------------------------------------------------------------

export interface DataGroupRow {
  id: number;
  sender_org_id: number;
  org_label: string;
  training_kind_id: number;
  training_kind: string;
  bzvp_program_id: number | null;
  vos_id: number | null;
  vos_label: string;
  position_id: number | null;
  course_id: number | null;
  site_id: number | null;
  venue_type: string | null;
  training_venue_id: number | null;
  city_id: number | null;
  site_label: string;
  city_label: string;
  organizer_org_id: number | null;
  organizer_label: string;
  planned_start: string;
  planned_end: string;
  equipment_text: string;
  basis_doc_number: string;
  basis_doc_date: string;
  note: string;
  planned_count: number;
  arrived_count: number;
  in_training_count: number;
  completed_count: number;
  attrition_count: number;
  discrepancy_count: number;
}

export interface GroupEventRow {
  id: number;
  group_id: number;
  event_type: string;
  count: number;
  occurred_on: string;
  recorded_at: string;
  reason_label: string | null;
  note: string | null;
  source_label: string | null;
  created_by_label: string | null;
  created_by_id: number | null;
  created_by_org: string | null;
  created_by_phone: string | null;
  created_by_rank: string | null;
  created_by_delta: string | null;
  created_by_active: boolean | null;
}

export interface TrainingKindOption {
  id: number;
  code: string;
  name: string;
}

export interface AttritionReasonOption {
  id: number;
  name: string;
}

export interface CreateGroupRequest {
  sender_org_id: number;
  training_kind_id: number;
  site_id?: number | null;
  venue_type?: string;
  training_venue_id?: number | null;
  city_id?: number | null;
  vos_id?: number | null;
  position_id?: number | null;
  course_id?: number | null;
  bzvp_program_id?: number | null;
  organizer_org_id?: number | null;
  planned_start: string;
  planned_end: string;
  equipment_text?: string;
  basis_doc_number?: string;
  basis_doc_date?: string;
  note?: string;
  planned_count: number;
  arrived_count?: number;
  in_training_count?: number;
}

export interface VenueRow {
  id: number;
  kind: string;
  name: string;
  short_name: string | null;
  military_number: string | null;
  city_id: number;
  city_name: string;
  org_id: number | null;
  is_active: boolean;
}

export interface VenueSearchRow {
  id: number;
  kind: string;
  name: string;
  city_id: number;
  city_name: string;
}

export interface CityRow {
  id: number;
  name: string;
}

export interface UpdateGroupRequest {
  sender_org_id?: number;
  training_kind_id?: number;
  site_id?: number;
  vos_id?: number | null;
  position_id?: number | null;
  course_id?: number | null;
  planned_start?: string;
  planned_end?: string;
  equipment_text?: string;
  note?: string;
  planned_count?: number;
}

export interface AddEventRequest {
  event_type: string;
  count: number;
  occurred_on: string;
  reason_id?: number | null;
  note?: string;
}

// ---------------------------------------------------------------------------
// Chat types
// ---------------------------------------------------------------------------

export interface ChatRoom {
  id: number;
  name: string;
  kind: string;
  emoji: string | null;
  unread_count: number | null;
  last_message_body: string | null;
  last_message_at: string | null;
  member_count: number | null;
}

export interface ChatMessage {
  id: number;
  room_id: number;
  sender_id: number;
  sender_label: string | null;
  sender_callsign: string | null;
  sender_avatar: string | null;
  sender_role: string | null;
  kind: string;
  body: string;
  media_url: string | null;
  media_mime: string | null;
  media_duration_sec: number | null;
  reply_to_id: number | null;
  reply_preview: string | null;
  created_at: string;
  updated_at: string | null;
  deleted_at?: string | null;
}

// ---------------------------------------------------------------------------
// Dictionary management types (admin)
// ---------------------------------------------------------------------------

export interface DictionaryEntry {
  id: number;
  label: string;
  extra: string | null;
  extra_id?: number;
}

export interface DictionariesOverview {
  training_kinds: DictionaryEntry[];
  training_directions: DictionaryEntry[];
  bzvp_programs: DictionaryEntry[];
  vos: DictionaryEntry[];
  positions: DictionaryEntry[];
  equipment: DictionaryEntry[];
  courses: DictionaryEntry[];
  attrition_reasons: DictionaryEntry[];
  training_sites: DictionaryEntry[];
}
