import type {
  AuthUser,
  LoginResponse,
  AdminUserRow,
  AdminGroupRow,
  AdminSubmissionRow,
  OrgDetail,
  OrgChild,
  DashboardStats,
  DiscrepancyRow,
  DiscrepancyComparison,
  AccountInfo,
  OrgSearchResult,
} from "./types";

const MOCK_USER: AuthUser = {
  user_id: 1,
  actor: { org_id: 1, role: "admin", org_label: 'УВ(с) "Південь"' },
  display_name: "Адміністратор",
};

const MOCK_CHILDREN: OrgChild[] = [
  { id: 2, label: "17 АК" },
  { id: 3, label: "20 АК" },
  { id: 4, label: "152 НЦ" },
  { id: 5, label: "30 КМП" },
];

const MOCK_GROUPS: AdminGroupRow[] = [
  { id: 1, org_label: "17 АК", training_kind: "Базова", vos_label: "011.01 Стрілець", site_label: "ЦБП Десна", planned_start: "01.10.2026", planned_end: "15.11.2026", planned_count: 120, arrived_count: 108, in_training_count: 102 },
  { id: 2, org_label: "17 АК", training_kind: "Фахова", vos_label: "141.01 Навідник ПТРК", site_label: "ЦБП Десна", planned_start: "05.10.2026", planned_end: "20.11.2026", planned_count: 45, arrived_count: 42, in_training_count: 40 },
  { id: 3, org_label: "20 АК", training_kind: "Базова", vos_label: "011.01 Стрілець", site_label: "ЦБП Рівне", planned_start: "10.10.2026", planned_end: "25.11.2026", planned_count: 90, arrived_count: 85, in_training_count: 78 },
  { id: 4, org_label: "20 АК", training_kind: "Спеціальна", vos_label: "904.01 Сапер", site_label: "НЦ Камʼянець", planned_start: "20.09.2026", planned_end: "30.10.2026", planned_count: 30, arrived_count: 30, in_training_count: 28 },
  { id: 5, org_label: "152 НЦ", training_kind: "Фахова", vos_label: "211.01 Командир гармати", site_label: "152 НЦ", planned_start: "15.10.2026", planned_end: "01.12.2026", planned_count: 60, arrived_count: 55, in_training_count: 50 },
  { id: 6, org_label: "30 КМП", training_kind: "Базова", vos_label: "011.01 Стрілець", site_label: "ЦБП Широкий лан", planned_start: "08.10.2026", planned_end: "22.11.2026", planned_count: 150, arrived_count: 140, in_training_count: 135 },
];

const MOCK_SUBMISSIONS: AdminSubmissionRow[] = [
  { id: 1, org_label: "17 АК", source_type: "Форма", status: "committed", as_of_date: "01.10.2026", updated_at: "01.10.2026 14:30" },
  { id: 2, org_label: "17 АК", source_type: "КВід", status: "committed", as_of_date: "01.10.2026", updated_at: "01.10.2026 15:12" },
  { id: 3, org_label: "20 АК", source_type: "Форма", status: "draft", as_of_date: "01.10.2026", updated_at: "02.10.2026 09:45" },
  { id: 4, org_label: "152 НЦ", source_type: "ІВС", status: "committed", as_of_date: "30.09.2026", updated_at: "30.09.2026 18:00" },
  { id: 5, org_label: "30 КМП", source_type: "Форма", status: "draft", as_of_date: "01.10.2026", updated_at: "02.10.2026 10:20" },
];

const MOCK_USERS: AdminUserRow[] = [
  { id: 1, login: "admin", display_name: "Адміністратор", is_active: true, must_change_password: false, roles: [{ org_id: 1, role: "admin", org_label: 'УВ(с) "Південь"' }], created_at: "01.09.2026" },
  { id: 7, login: "editor_17ak", display_name: "Редактор 17 АК", is_active: true, must_change_password: true, roles: [{ org_id: 2, role: "org_editor", org_label: "17 АК" }], created_at: "15.09.2026" },
  { id: 8, login: "editor_20ak", display_name: "Редактор 20 АК", is_active: true, must_change_password: true, roles: [{ org_id: 3, role: "org_editor", org_label: "20 АК" }], created_at: "15.09.2026" },
  { id: 9, login: "viewer_pivden", display_name: "Спостерігач УВ(с)", is_active: true, must_change_password: true, roles: [{ org_id: 1, role: "viewer", org_label: 'УВ(с) "Південь"' }], created_at: "20.09.2026" },
  { id: 10, login: "editor_152nc", display_name: "Редактор 152 НЦ", is_active: true, must_change_password: false, roles: [{ org_id: 4, role: "org_editor", org_label: "152 НЦ" }], created_at: "22.09.2026" },
  { id: 11, login: "viewer_30kmp", display_name: "Спостерігач 30 КМП", is_active: false, must_change_password: true, roles: [{ org_id: 5, role: "viewer", org_label: "30 КМП" }], created_at: "25.09.2026" },
];

const MOCK_DISCREPANCIES: DiscrepancyRow[] = [
  { id: 1, kind: "horizontal", org_id: 2, org_label: "17 АК", group_id: 1, group_label: "Базова · ВОС 011.01 · з 01.10.2026", as_of: "2026-10-01", metric: "planned_count", metric_label: "План", values: [{ submission_id: 1, source_label: "128 овмбр · Форма", value: "120" }, { submission_id: 2, source_label: "17 АК · Таблиця", value: "115" }], status: "open", created_at: "2026-10-01 14:35" },
  { id: 2, kind: "horizontal", org_id: 3, org_label: "20 АК", group_id: 3, group_label: "Базова · ВОС 011.01 · з 10.10.2026", as_of: "2026-10-01", metric: "arrived_count", metric_label: "Прибуло", values: [{ submission_id: 3, source_label: "65 омбр · Форма", value: "85" }, { submission_id: 4, source_label: "20 АК · Таблиця", value: "90" }], status: "open", created_at: "2026-10-01 15:20" },
  { id: 3, kind: "temporal", org_id: 4, org_label: "152 НЦ", group_id: 5, group_label: "Фахова · ВОС 211.01 · з 15.10.2026", as_of: "2026-09-30", metric: "planned_start", metric_label: "Термін з", values: [{ submission_id: 4, source_label: "152 НЦ · Форма", value: "2026-10-15" }], status: "resolved", created_at: "2026-09-30 18:05" },
  { id: 4, kind: "horizontal", org_id: 5, org_label: "30 КМП", group_id: 6, group_label: "Базова · ВОС 011.01 · з 08.10.2026", as_of: "2026-10-01", metric: "planned_end", metric_label: "Термін по", values: [{ submission_id: 5, source_label: "30 КМП · Форма", value: "2026-11-22" }, { submission_id: 4, source_label: "152 НЦ · Форма", value: "2026-11-30" }], status: "open", created_at: "2026-10-02 10:25" },
];

const MOCK_STATS: DashboardStats = {
  total_orgs: 12,
  total_groups: 6,
  total_submissions: 5,
  total_discrepancies: 4,
  groups_by_kind: [
    { kind: "bzvp", count: 3 },
    { kind: "special", count: 2 },
    { kind: "adaptation", count: 1 },
  ],
  recent_submissions: [],
  recent_discrepancies: [],
};

const MOCK_ACCOUNT: AccountInfo = {
  login: "admin",
  full_name: "Адміністратор",
  roles: ["admin (УВ(с) \"Південь\")"],
  created_at: "2026-09-01",
};

const ORG_MAP: Record<number, OrgDetail> = {
  1: { id: 1, short_name: 'УВ(с) "Південь"', full_name: 'Управління Військ (сил) "Південь"', kind: "Управління", is_active: true, name_history: [], status_history: [] },
  2: { id: 2, short_name: "17 АК", full_name: "17 армійський корпус", kind: "Корпус", is_active: true, name_history: [], status_history: [] },
  3: { id: 3, short_name: "20 АК", full_name: "20 армійський корпус", kind: "Корпус", is_active: true, name_history: [], status_history: [] },
  4: { id: 4, short_name: "152 НЦ", full_name: "152 навчальний центр", kind: "Навчальний центр", is_active: true, name_history: [], status_history: [] },
  5: { id: 5, short_name: "30 КМП", full_name: "30 корпус морської піхоти", kind: "Корпус", is_active: true, name_history: [], status_history: [] },
};

function orgLabel(id: number): string {
  const org = ORG_MAP[id];
  if (!org) return `Org #${id}`;
  return org.short_name;
}

type MockHandler = (url: string, body?: unknown) => unknown | null;

const routes: MockHandler[] = [
  (url) => { if (url === "/api/auth/me") return MOCK_USER; return null; },
  (url, body) => {
    if (url === "/api/auth/login") {
      const b = body as { login: string; password: string } | undefined;
      if (b?.login === "admin" && (b?.password === "admin123" || b?.password === "admin")) {
        return { success: true, error: null, display_name: "Адміністратор", roles: MOCK_USER.actor ? [{ org_id: MOCK_USER.actor.org_id, role: MOCK_USER.actor.role, org_label: MOCK_USER.actor.org_label }] : [], must_change_password: false } satisfies LoginResponse;
      }
      return { success: false, error: "Невірний логін або пароль", display_name: null, roles: [], must_change_password: false } satisfies LoginResponse;
    }
    return null;
  },
  (url) => { if (url === "/api/auth/logout") return {}; return null; },
  (url) => { if (url === "/api/auth/account") return MOCK_ACCOUNT; return null; },
  (url) => { if (url === "/api/dashboard/stats") return MOCK_STATS; return null; },
  (url) => {
    if (url === "/api/dashboard/pipeline") return { planned: 495, arrived: 460, in_training: 433, completed: 312, attrition: 47 };
    return null;
  },
  (url) => {
    if (url === "/api/dashboard/by-org") return [
      { org_name: "17 АК", group_count: 2, planned: 165, completed: 95, attrition: 12 },
      { org_name: "30 КМП", group_count: 1, planned: 150, completed: 98, attrition: 15 },
      { org_name: "20 АК", group_count: 2, planned: 120, completed: 72, attrition: 11 },
      { org_name: "152 НЦ", group_count: 1, planned: 60, completed: 47, attrition: 9 },
    ];
    return null;
  },
  (url) => {
    if (url === "/api/dashboard/timeline") return [
      { week: "2026-08-24", planned: 60, started: 45, completed: 20, attrition: 3 },
      { week: "2026-08-31", planned: 80, started: 72, completed: 35, attrition: 5 },
      { week: "2026-09-07", planned: 95, started: 88, completed: 50, attrition: 7 },
      { week: "2026-09-14", planned: 110, started: 100, completed: 65, attrition: 8 },
      { week: "2026-09-21", planned: 75, started: 70, completed: 72, attrition: 10 },
      { week: "2026-09-28", planned: 75, started: 85, completed: 70, attrition: 14 },
    ];
    return null;
  },
  (url) => {
    if (url === "/api/dashboard/discrepancies-chart") return {
      by_status: [{ status: "open", count: 3 }, { status: "resolved", count: 1 }, { status: "accepted", count: 0 }],
      by_org: [
        { org_name: "17 АК", open: 1, resolved: 0, accepted: 0 },
        { org_name: "20 АК", open: 1, resolved: 0, accepted: 0 },
        { org_name: "30 КМП", open: 1, resolved: 0, accepted: 0 },
        { org_name: "152 НЦ", open: 0, resolved: 1, accepted: 0 },
      ],
    };
    return null;
  },
  (url) => {
    if (url === "/api/dashboard/staffing") return [
      { org_name: "17 АК", category: "squad_leaders", authorized: 48, assigned: 42 },
      { org_name: "17 АК", category: "instructors", authorized: 24, assigned: 20 },
      { org_name: "20 АК", category: "squad_leaders", authorized: 36, assigned: 28 },
      { org_name: "20 АК", category: "instructors", authorized: 18, assigned: 15 },
      { org_name: "152 НЦ", category: "squad_leaders", authorized: 20, assigned: 18 },
      { org_name: "152 НЦ", category: "instructors", authorized: 12, assigned: 11 },
      { org_name: "30 КМП", category: "squad_leaders", authorized: 40, assigned: 25 },
      { org_name: "30 КМП", category: "instructors", authorized: 20, assigned: 12 },
    ];
    return null;
  },
  (url) => {
    const m = url.match(/^\/api\/orgs\/(\d+)$/);
    if (m) {
      const id = parseInt(m[1]);
      return ORG_MAP[id] ?? { id, short_name: `Org #${id}`, full_name: null, number: null, kind: "unknown", is_active: true, name_history: [], status_history: [] };
    }
    return null;
  },
  (url) => {
    const m = url.match(/^\/api\/orgs\/(\d+)\/children$/);
    if (m) {
      const id = parseInt(m[1]);
      if (id === 1) return MOCK_CHILDREN;
      return [];
    }
    return null;
  },
  (url) => {
    const m = url.match(/^\/api\/orgs\/(\d+)\/groups$/);
    if (m) {
      const id = parseInt(m[1]);
      if (id === 1) return MOCK_GROUPS;
      return MOCK_GROUPS.filter(g => g.org_label === orgLabel(id));
    }
    return null;
  },
  (url) => {
    const m = url.match(/^\/api\/orgs\/(\d+)\/submissions$/);
    if (m) {
      const id = parseInt(m[1]);
      if (id === 1) return MOCK_SUBMISSIONS;
      return MOCK_SUBMISSIONS.filter(s => s.org_label === orgLabel(id));
    }
    return null;
  },
  (url) => {
    const m = url.match(/^\/api\/orgs\/(\d+)\/users$/);
    if (m) return MOCK_USERS;
    return null;
  },
  (url) => { if (url === "/api/discrepancies") return MOCK_DISCREPANCIES; return null; },
  (url) => {
    const m = url.match(/^\/api\/discrepancies\/(\d+)\/compare$/);
    if (m) {
      const id = parseInt(m[1]);
      const disc = MOCK_DISCREPANCIES.find(d => d.id === id);
      if (!disc) return null;
      const comparison: DiscrepancyComparison = {
        metric: disc.metric,
        metric_label: disc.metric_label,
        rows: disc.values.map(v => ({
          submission_id: v.submission_id,
          source_label: v.source_label,
          training_kind: disc.group_label?.split(" · ")[0] ?? "Базова",
          vos_label: disc.group_label?.match(/ВОС (\S+)/)?.[1] ?? null,
          position_label: null,
          course_label: null,
          site_label: "ЦБП Десна",
          organizer_label: null,
          planned_start: disc.group_label?.match(/з (\S+)/)?.[1] ?? "01.10.2026",
          planned_end: "15.11.2026",
          equipment_text: null,
          basis_doc_number: null,
          note: null,
          planned_count: parseInt(disc.metric === "planned_count" ? v.value : "120"),
          arrived_count: parseInt(disc.metric === "arrived_count" ? v.value : "108"),
          in_training_count: parseInt(disc.metric === "in_training_count" ? v.value : "102"),
        })),
      };
      return comparison;
    }
    return null;
  },
  (url) => { if (url === "/api/admin/users") return MOCK_USERS; return null; },
  (url) => { if (url === "/api/training/groups") return MOCK_GROUPS; return null; },
  (url) => { if (url === "/api/submissions/recent") return MOCK_SUBMISSIONS; return null; },
  (url) => {
    if (url.startsWith("/api/orgs/search")) {
      const results: OrgSearchResult[] = Object.values(ORG_MAP).map(o => ({
        org_id: o.id,
        label: o.short_name,
        matched_raw: o.short_name,
        is_exact: true,
      }));
      return results;
    }
    return null;
  },
];

export function mockFetch(url: string, body?: unknown): unknown | undefined {
  for (const handler of routes) {
    const result = handler(url, body);
    if (result !== null) return result;
  }
  return undefined;
}
