import { useEffect, useState, useMemo, useCallback } from "react";
import { useSearchParams, useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { api } from "@/api/client";
import type { ChatRoom } from "@/api/types";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { DatePickerUa } from "@/components/ui/date-picker-ua";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  DropdownMenuSub,
  DropdownMenuSubTrigger,
  DropdownMenuSubContent,
} from "@/components/ui/dropdown-menu";
import {
  Building2,
  Users,
  AlertTriangle,
  TrendingUp,
  Target,
  Activity,
  BarChart3,
  PieChart as PieIcon,
  Calendar,
  Shield,
  Filter,
  X,
  RotateCcw,
  Share2,
  MessageSquare,
  Copy,
} from "lucide-react";
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
  AreaChart,
  Area,
  Legend,
  RadialBarChart,
  RadialBar,
} from "recharts";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface PipelineData {
  planned: number;
  arrived: number;
  in_training: number;
  completed: number;
  attrition: number;
}

interface OrgData {
  org_name: string;
  group_count: number;
  planned: number;
  completed: number;
  attrition: number;
}

interface TimelinePoint {
  week: string;
  planned: number;
  started: number;
  completed: number;
  attrition: number;
}

interface DiscData {
  by_status: { status: string; count: number }[];
  by_org: { org_name: string; open: number; resolved: number; accepted: number }[];
}

interface StaffingRow {
  org_name: string;
  category: string;
  authorized: number;
  assigned: number;
}

interface DashboardStats {
  total_orgs: number;
  total_groups: number;
  total_submissions: number;
  total_discrepancies: number;
  groups_by_kind: { kind: string; count: number }[];
}

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

const CHART_COLORS = {
  gold: "#c9a84c",
  goldLight: "rgba(201,168,76,0.6)",
  green: "#8fa565",
  greenLight: "rgba(143,165,101,0.6)",
  blue: "#5B9BD5",
  blueLight: "rgba(91,155,213,0.6)",
  red: "#c97a6e",
  redLight: "rgba(201,122,110,0.6)",
  purple: "#9b8ec4",
  teal: "#5bb5a6",
  orange: "#d4913d",
};

const KIND_LABELS: Record<string, string> = {
  bzvp: "БЗВП",
  special: "Фахова",
  adaptation: "Адаптація",
  internship: "Стажування",
};

const KIND_COLORS: Record<string, string> = {
  bzvp: CHART_COLORS.green,
  special: CHART_COLORS.blue,
  adaptation: CHART_COLORS.gold,
  internship: CHART_COLORS.red,
};

const STATUS_LABELS: Record<string, string> = {
  open: "Відкриті",
  in_progress: "В роботі",
  resolved: "Вирішені",
  accepted: "Прийняті",
  dismissed: "Відхилені",
  notified: "Повідомлено",
};

const STATUS_COLORS: Record<string, string> = {
  open: CHART_COLORS.red,
  in_progress: CHART_COLORS.orange,
  resolved: CHART_COLORS.green,
  accepted: CHART_COLORS.blue,
  dismissed: "#888",
  notified: CHART_COLORS.teal,
};

const PIE_COLORS = [CHART_COLORS.red, CHART_COLORS.green, CHART_COLORS.blue, CHART_COLORS.gold];

const ALL_KINDS = ["bzvp", "special", "adaptation", "internship"] as const;

// ---------------------------------------------------------------------------
// Filter types & component
// ---------------------------------------------------------------------------

interface DashFilters {
  orgs: Set<string>;
  kinds: Set<string>;
  dateFrom: string;
  dateTo: string;
}

const EMPTY_FILTERS: DashFilters = { orgs: new Set(), kinds: new Set(), dateFrom: "", dateTo: "" };

function toggleSet<T>(set: Set<T>, val: T): Set<T> {
  const next = new Set(set);
  if (next.has(val)) next.delete(val);
  else next.add(val);
  return next;
}

function FilterPill({
  label,
  active,
  color,
  onClick,
}: {
  label: string;
  active: boolean;
  color: string;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className="relative rounded-full px-3.5 py-1.5 text-xs font-semibold transition-all duration-300 cursor-pointer select-none outline-none focus-visible:ring-2 focus-visible:ring-primary/50"
      style={{
        background: active ? `${color}25` : "var(--muted)",
        color: active ? color : "var(--muted-foreground)",
        border: `1.5px solid ${active ? color : "transparent"}`,
        boxShadow: active ? `0 0 12px ${color}30, inset 0 0 12px ${color}10` : "none",
        transform: active ? "scale(1.05)" : "scale(1)",
      }}
    >
      {active && (
        <span
          className="absolute inset-0 rounded-full animate-pulse"
          style={{ background: `${color}08` }}
        />
      )}
      <span className="relative">{label}</span>
    </button>
  );
}

function DashboardFilters({
  filters,
  onChange,
  availableOrgs,
}: {
  filters: DashFilters;
  onChange: (f: DashFilters) => void;
  availableOrgs: string[];
}) {
  const activeCount =
    filters.orgs.size + filters.kinds.size + (filters.dateFrom ? 1 : 0) + (filters.dateTo ? 1 : 0);

  const reset = () => onChange(EMPTY_FILTERS);

  return (
    <Card className="card-animate overflow-hidden">
      <CardContent className="flex flex-col gap-4 p-4">
        {/* Row 1: Orgs */}
        <div className="flex flex-wrap items-center gap-2">
          <span className="flex items-center gap-1.5 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground shrink-0 mr-1">
            <Building2 className="h-3.5 w-3.5" />
            Підрозділ
          </span>
          {availableOrgs.map((org) => (
            <FilterPill
              key={org}
              label={org}
              active={filters.orgs.has(org)}
              color={CHART_COLORS.gold}
              onClick={() => onChange({ ...filters, orgs: toggleSet(filters.orgs, org) })}
            />
          ))}
        </div>

        {/* Row 2: Kinds + Date range + Reset */}
        <div className="flex flex-wrap items-center gap-x-6 gap-y-3">
          <div className="flex flex-wrap items-center gap-2">
            <span className="flex items-center gap-1.5 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground shrink-0 mr-1">
              <Target className="h-3.5 w-3.5" />
              Вид
            </span>
            {ALL_KINDS.map((k) => (
              <FilterPill
                key={k}
                label={KIND_LABELS[k] || k}
                active={filters.kinds.has(k)}
                color={KIND_COLORS[k] || CHART_COLORS.purple}
                onClick={() => onChange({ ...filters, kinds: toggleSet(filters.kinds, k) })}
              />
            ))}
          </div>

          <div className="flex items-center gap-2">
            <span className="flex items-center gap-1.5 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground shrink-0 mr-1">
              <Calendar className="h-3.5 w-3.5" />
              Період
            </span>
            <DatePickerUa
              value={filters.dateFrom}
              onChange={(v) => onChange({ ...filters, dateFrom: v })}
              placeholder="З"
              className="h-8 w-24 sm:w-32 text-xs"
            />
            <span className="text-muted-foreground text-xs">—</span>
            <DatePickerUa
              value={filters.dateTo}
              onChange={(v) => onChange({ ...filters, dateTo: v })}
              placeholder="По"
              className="h-8 w-24 sm:w-32 text-xs"
            />
          </div>

          {activeCount > 0 && (
            <Button
              variant="ghost"
              size="sm"
              onClick={reset}
              className="h-8 gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors"
            >
              <RotateCcw className="h-3.5 w-3.5" />
              Скинути
              <Badge
                variant="outline"
                className="ml-0.5 h-4 min-w-4 px-1 text-[10px] tabular-nums"
                style={{
                  borderColor: CHART_COLORS.gold,
                  color: CHART_COLORS.gold,
                }}
              >
                {activeCount}
              </Badge>
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// Custom tooltip
// ---------------------------------------------------------------------------

function ChartTooltip({ active, payload, label }: any) {
  if (!active || !payload?.length) return null;
  return (
    <div
      className="rounded-lg px-3 py-2 text-xs shadow-xl"
      style={{
        background: "var(--card)",
        border: "1px solid var(--border)",
        color: "var(--foreground)",
      }}
    >
      {label && <p className="font-semibold mb-1">{label}</p>}
      {payload.map((p: any, i: number) => (
        <div key={i} className="flex items-center gap-2">
          <span className="h-2 w-2 rounded-full" style={{ background: p.color }} />
          <span className="text-muted-foreground">{p.name}:</span>
          <span className="font-bold">{p.value?.toLocaleString("uk-UA")}</span>
        </div>
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Stat card
// ---------------------------------------------------------------------------

function StatCard({
  title,
  value,
  icon: Icon,
  color,
  subtitle,
}: {
  title: string;
  value: number | string;
  icon: React.ElementType;
  color: string;
  subtitle?: string;
}) {
  return (
    <Card className="card-animate card-hover">
      <CardContent className="flex items-center gap-4 p-4">
        <div
          className="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl"
          style={{
            background: `linear-gradient(135deg, ${color}20, ${color}10)`,
            border: `1px solid ${color}30`,
          }}
        >
          <Icon className="h-5 w-5" style={{ color }} />
        </div>
        <div className="min-w-0">
          <p className="text-2xl font-bold tabular-nums" style={{ color }}>
            {typeof value === "number" ? value.toLocaleString("uk-UA") : value}
          </p>
          <p className="text-xs text-muted-foreground truncate">{title}</p>
          {subtitle && (
            <p className="text-[10px] mt-0.5" style={{ color: `${color}99` }}>
              {subtitle}
            </p>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// Pipeline funnel
// ---------------------------------------------------------------------------

function PipelineChart({ data }: { data: PipelineData }) {
  const stages = [
    { name: "Заплановано", value: data.planned, color: CHART_COLORS.blue },
    { name: "Прибуло", value: data.arrived, color: CHART_COLORS.teal },
    { name: "Навчається", value: data.in_training, color: CHART_COLORS.gold },
    { name: "Завершило", value: data.completed, color: CHART_COLORS.green },
    { name: "Вибуло", value: data.attrition, color: CHART_COLORS.red },
  ];

  const maxVal = Math.max(...stages.map((s) => s.value), 1);

  return (
    <div className="flex flex-col gap-3">
      {stages.map((stage, i) => {
        const pct = maxVal > 0 ? (stage.value / maxVal) * 100 : 0;
        return (
          <div key={i} className="flex items-center gap-3">
            <span className="w-24 text-xs text-right text-muted-foreground shrink-0">
              {stage.name}
            </span>
            <div className="flex-1 h-8 rounded-lg overflow-hidden" style={{ background: "var(--muted)" }}>
              <div
                className="h-full rounded-lg flex items-center px-3 text-xs font-bold transition-all duration-700"
                style={{
                  width: `${Math.max(pct, 2)}%`,
                  background: `linear-gradient(90deg, ${stage.color}, ${stage.color}cc)`,
                  color: "#14140c",
                  boxShadow: `0 0 12px ${stage.color}40`,
                }}
              >
                {stage.value > 0 ? stage.value.toLocaleString("uk-UA") : ""}
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Training by kind donut
// ---------------------------------------------------------------------------

function KindDonut({ data }: { data: { kind: string; count: number }[] }) {
  const chartData = data.map((d) => ({
    name: KIND_LABELS[d.kind] || d.kind,
    value: d.count,
    color: KIND_COLORS[d.kind] || CHART_COLORS.purple,
  }));

  const total = chartData.reduce((s, d) => s + d.value, 0);

  return (
    <div className="flex items-center gap-4">
      <ResponsiveContainer width={160} height={160}>
        <PieChart>
          <Pie
            data={chartData}
            cx="50%"
            cy="50%"
            innerRadius={45}
            outerRadius={70}
            paddingAngle={3}
            dataKey="value"
            strokeWidth={0}
          >
            {chartData.map((d, i) => (
              <Cell key={i} fill={d.color} />
            ))}
          </Pie>
          <Tooltip content={<ChartTooltip />} />
        </PieChart>
      </ResponsiveContainer>
      <div className="flex flex-col gap-2 flex-1">
        {chartData.map((d, i) => (
          <div key={i} className="flex items-center gap-2">
            <span className="h-3 w-3 rounded-sm shrink-0" style={{ background: d.color }} />
            <span className="text-xs flex-1">{d.name}</span>
            <span className="text-xs font-bold tabular-nums">{d.value}</span>
            <span className="text-[10px] text-muted-foreground w-10 text-right tabular-nums">
              {total > 0 ? Math.round((d.value / total) * 100) : 0}%
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Org bar chart
// ---------------------------------------------------------------------------

function OrgBarChart({ data }: { data: OrgData[] }) {
  return (
    <ResponsiveContainer width="100%" height={300}>
      <BarChart data={data} layout="vertical" margin={{ left: 0, right: 16, top: 8, bottom: 8 }}>
        <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" opacity={0.3} />
        <XAxis type="number" tick={{ fill: "var(--muted-foreground)", fontSize: 11 }} />
        <YAxis
          dataKey="org_name"
          type="category"
          width={90}
          tick={{ fill: "var(--foreground)", fontSize: 11 }}
        />
        <Tooltip content={<ChartTooltip />} />
        <Legend
          wrapperStyle={{ fontSize: 11 }}
          formatter={(v: string) => <span style={{ color: "var(--foreground)" }}>{v}</span>}
        />
        <Bar dataKey="planned" name="Заплановано" fill={CHART_COLORS.blue} radius={[0, 4, 4, 0]} />
        <Bar dataKey="completed" name="Завершило" fill={CHART_COLORS.green} radius={[0, 4, 4, 0]} />
        <Bar dataKey="attrition" name="Вибуло" fill={CHART_COLORS.red} radius={[0, 4, 4, 0]} />
      </BarChart>
    </ResponsiveContainer>
  );
}

// ---------------------------------------------------------------------------
// Timeline area chart
// ---------------------------------------------------------------------------

function TimelineChart({ data }: { data: TimelinePoint[] }) {
  const formatted = data.map((d) => ({
    ...d,
    label: new Date(d.week).toLocaleDateString("uk-UA", { day: "numeric", month: "short" }),
  }));

  return (
    <ResponsiveContainer width="100%" height={280}>
      <AreaChart data={formatted} margin={{ left: 0, right: 16, top: 8, bottom: 8 }}>
        <defs>
          <linearGradient id="gradPlanned" x1="0" y1="0" x2="0" y2="1">
            <stop offset="5%" stopColor={CHART_COLORS.blue} stopOpacity={0.3} />
            <stop offset="95%" stopColor={CHART_COLORS.blue} stopOpacity={0.02} />
          </linearGradient>
          <linearGradient id="gradStarted" x1="0" y1="0" x2="0" y2="1">
            <stop offset="5%" stopColor={CHART_COLORS.gold} stopOpacity={0.3} />
            <stop offset="95%" stopColor={CHART_COLORS.gold} stopOpacity={0.02} />
          </linearGradient>
          <linearGradient id="gradCompleted" x1="0" y1="0" x2="0" y2="1">
            <stop offset="5%" stopColor={CHART_COLORS.green} stopOpacity={0.3} />
            <stop offset="95%" stopColor={CHART_COLORS.green} stopOpacity={0.02} />
          </linearGradient>
        </defs>
        <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" opacity={0.3} />
        <XAxis dataKey="label" tick={{ fill: "var(--muted-foreground)", fontSize: 10 }} />
        <YAxis tick={{ fill: "var(--muted-foreground)", fontSize: 11 }} />
        <Tooltip content={<ChartTooltip />} />
        <Legend
          wrapperStyle={{ fontSize: 11 }}
          formatter={(v: string) => <span style={{ color: "var(--foreground)" }}>{v}</span>}
        />
        <Area
          type="monotone"
          dataKey="planned"
          name="Заплановано"
          stroke={CHART_COLORS.blue}
          fill="url(#gradPlanned)"
          strokeWidth={2}
        />
        <Area
          type="monotone"
          dataKey="started"
          name="Розпочато"
          stroke={CHART_COLORS.gold}
          fill="url(#gradStarted)"
          strokeWidth={2}
        />
        <Area
          type="monotone"
          dataKey="completed"
          name="Завершено"
          stroke={CHART_COLORS.green}
          fill="url(#gradCompleted)"
          strokeWidth={2}
        />
      </AreaChart>
    </ResponsiveContainer>
  );
}

// ---------------------------------------------------------------------------
// Discrepancy charts
// ---------------------------------------------------------------------------

function DiscStatusDonut({ data }: { data: { status: string; count: number }[] }) {
  const chartData = data.map((d) => ({
    name: STATUS_LABELS[d.status] || d.status,
    value: d.count,
    color: STATUS_COLORS[d.status] || CHART_COLORS.purple,
  }));
  const total = chartData.reduce((s, d) => s + d.value, 0);

  return (
    <div className="flex items-center gap-4">
      <ResponsiveContainer width={140} height={140}>
        <PieChart>
          <Pie
            data={chartData}
            cx="50%"
            cy="50%"
            innerRadius={38}
            outerRadius={60}
            paddingAngle={4}
            dataKey="value"
            strokeWidth={0}
          >
            {chartData.map((d, i) => (
              <Cell key={i} fill={d.color} />
            ))}
          </Pie>
          <Tooltip content={<ChartTooltip />} />
        </PieChart>
      </ResponsiveContainer>
      <div className="flex flex-col gap-2">
        <p className="text-2xl font-bold" style={{ color: "var(--foreground)" }}>
          {total}
        </p>
        {chartData.map((d, i) => (
          <div key={i} className="flex items-center gap-2">
            <span className="h-2.5 w-2.5 rounded-full" style={{ background: d.color }} />
            <span className="text-xs">{d.name}</span>
            <span className="text-xs font-bold tabular-nums">{d.value}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function DiscOrgChart({ data }: { data: { org_name: string; open: number; resolved: number; accepted: number }[] }) {
  return (
    <ResponsiveContainer width="100%" height={250}>
      <BarChart data={data} margin={{ left: 0, right: 16, top: 8, bottom: 8 }}>
        <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" opacity={0.3} />
        <XAxis dataKey="org_name" tick={{ fill: "var(--muted-foreground)", fontSize: 10 }} />
        <YAxis tick={{ fill: "var(--muted-foreground)", fontSize: 11 }} />
        <Tooltip content={<ChartTooltip />} />
        <Legend
          wrapperStyle={{ fontSize: 11 }}
          formatter={(v: string) => <span style={{ color: "var(--foreground)" }}>{v}</span>}
        />
        <Bar dataKey="open" name="Відкриті" fill={CHART_COLORS.red} stackId="a" radius={[0, 0, 0, 0]} />
        <Bar dataKey="resolved" name="Вирішені" fill={CHART_COLORS.green} stackId="a" radius={[0, 0, 0, 0]} />
        <Bar dataKey="accepted" name="Прийняті" fill={CHART_COLORS.blue} stackId="a" radius={[4, 4, 0, 0]} />
      </BarChart>
    </ResponsiveContainer>
  );
}

// ---------------------------------------------------------------------------
// Staffing gauge
// ---------------------------------------------------------------------------

function StaffingChart({ data }: { data: StaffingRow[] }) {
  const grouped = useMemo(() => {
    const map = new Map<string, { authorized: number; assigned: number }>();
    for (const row of data) {
      const key = row.org_name;
      const prev = map.get(key) || { authorized: 0, assigned: 0 };
      map.set(key, {
        authorized: prev.authorized + row.authorized,
        assigned: prev.assigned + row.assigned,
      });
    }
    return Array.from(map, ([org_name, vals]) => ({
      org_name,
      ...vals,
      fill_pct: vals.authorized > 0 ? Math.round((vals.assigned / vals.authorized) * 100) : 0,
    })).sort((a, b) => b.fill_pct - a.fill_pct);
  }, [data]);

  if (grouped.length === 0) {
    return <p className="text-sm text-muted-foreground italic">Немає даних укомплектованості</p>;
  }

  return (
    <div className="flex flex-col gap-2">
      {grouped.map((g, i) => {
        const pct = g.fill_pct;
        const barColor =
          pct >= 80 ? CHART_COLORS.green : pct >= 50 ? CHART_COLORS.gold : CHART_COLORS.red;
        return (
          <div key={i} className="flex items-center gap-2">
            <span className="w-20 text-xs text-right text-muted-foreground truncate shrink-0">
              {g.org_name}
            </span>
            <div className="flex-1 h-5 rounded overflow-hidden" style={{ background: "var(--muted)" }}>
              <div
                className="h-full rounded flex items-center justify-end px-1.5 text-[10px] font-bold transition-all duration-500"
                style={{
                  width: `${Math.max(pct, 3)}%`,
                  background: `linear-gradient(90deg, ${barColor}cc, ${barColor})`,
                  color: "#14140c",
                }}
              >
                {pct}%
              </div>
            </div>
            <span className="text-[10px] text-muted-foreground tabular-nums w-16 shrink-0">
              {g.assigned}/{g.authorized}
            </span>
          </div>
        );
      })}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Section wrapper
// ---------------------------------------------------------------------------

function ChartSection({
  title,
  icon: Icon,
  children,
  className,
}: {
  title: string;
  icon: React.ElementType;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <Card className={`card-animate ${className ?? ""}`}>
      <CardHeader className="pb-2">
        <CardTitle className="flex items-center gap-2 text-sm">
          <div className="icon-box" style={{ width: 28, height: 28 }}>
            <Icon className="h-3.5 w-3.5" />
          </div>
          {title}
        </CardTitle>
      </CardHeader>
      <CardContent>{children}</CardContent>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// Main page
// ---------------------------------------------------------------------------

export function DashboardPage() {
  const [searchParams, setSearchParams] = useSearchParams();
  const navigate = useNavigate();
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [pipeline, setPipeline] = useState<PipelineData | null>(null);
  const [orgData, setOrgData] = useState<OrgData[]>([]);
  const [timeline, setTimeline] = useState<TimelinePoint[]>([]);
  const [discData, setDiscData] = useState<DiscData | null>(null);
  const [staffing, setStaffing] = useState<StaffingRow[]>([]);
  const [loading, setLoading] = useState(true);

  const initFilters = useMemo((): DashFilters => {
    const orgs = searchParams.get("orgs");
    const kinds = searchParams.get("kinds");
    return {
      orgs: orgs ? new Set(orgs.split(",")) : new Set(),
      kinds: kinds ? new Set(kinds.split(",")) : new Set(),
      dateFrom: searchParams.get("from") ?? "",
      dateTo: searchParams.get("to") ?? "",
    };
  }, []);
  const [filters, setFilters] = useState<DashFilters>(initFilters);

  useEffect(() => {
    const params: Record<string, string> = {};
    if (filters.orgs.size) params.orgs = [...filters.orgs].join(",");
    if (filters.kinds.size) params.kinds = [...filters.kinds].join(",");
    if (filters.dateFrom) params.from = filters.dateFrom;
    if (filters.dateTo) params.to = filters.dateTo;
    setSearchParams(params, { replace: true });
  }, [filters, setSearchParams]);

  const hasFilters = filters.orgs.size > 0 || filters.kinds.size > 0 || !!filters.dateFrom || !!filters.dateTo;

  const buildShareUrl = useCallback(() => {
    const params = new URLSearchParams();
    if (filters.orgs.size) params.set("orgs", [...filters.orgs].join(","));
    if (filters.kinds.size) params.set("kinds", [...filters.kinds].join(","));
    if (filters.dateFrom) params.set("from", filters.dateFrom);
    if (filters.dateTo) params.set("to", filters.dateTo);
    const qs = params.toString();
    return `/dashboard${qs ? `?${qs}` : ""}`;
  }, [filters]);

  const [chatRooms, setChatRooms] = useState<ChatRoom[]>([]);
  const [roomsLoaded, setRoomsLoaded] = useState(false);
  const loadRooms = useCallback(() => {
    if (roomsLoaded) return;
    api.get<ChatRoom[]>("/chat/rooms").then(setChatRooms).catch(() => {});
    setRoomsLoaded(true);
  }, [roomsLoaded]);

  useEffect(() => {
    Promise.allSettled([
      api.get<DashboardStats>("/dashboard/stats"),
      api.get<PipelineData>("/dashboard/pipeline"),
      api.get<OrgData[]>("/dashboard/by-org"),
      api.get<TimelinePoint[]>("/dashboard/timeline"),
      api.get<DiscData>("/dashboard/discrepancies-chart"),
      api.get<StaffingRow[]>("/dashboard/staffing"),
    ]).then(([s, p, o, t, d, st]) => {
      if (s.status === "fulfilled") setStats(s.value);
      if (p.status === "fulfilled") setPipeline(p.value);
      if (o.status === "fulfilled") setOrgData(o.value);
      if (t.status === "fulfilled") setTimeline(t.value);
      if (d.status === "fulfilled") setDiscData(d.value);
      if (st.status === "fulfilled") setStaffing(st.value);
      setLoading(false);
    });
  }, []);

  const availableOrgs = useMemo(
    () => Array.from(new Set([
      ...orgData.map((o) => o.org_name),
      ...(discData?.by_org.map((o) => o.org_name) ?? []),
      ...staffing.map((s) => s.org_name),
    ])).sort(),
    [orgData, discData, staffing],
  );

  const hasOrgFilter = filters.orgs.size > 0;
  const hasKindFilter = filters.kinds.size > 0;

  const filteredOrgData = useMemo(() => {
    if (!hasOrgFilter) return orgData;
    return orgData.filter((o) => filters.orgs.has(o.org_name));
  }, [orgData, filters.orgs, hasOrgFilter]);

  const filteredDiscData = useMemo((): DiscData | null => {
    if (!discData) return null;
    if (!hasOrgFilter) return discData;
    const byOrg = discData.by_org.filter((o) => filters.orgs.has(o.org_name));
    const byStatus = discData.by_status.map((s) => {
      const key = s.status as "open" | "resolved" | "accepted";
      const count = byOrg.reduce((sum, o) => sum + (o[key] ?? 0), 0);
      return { ...s, count };
    });
    return { by_status: byStatus, by_org: byOrg };
  }, [discData, filters.orgs, hasOrgFilter]);

  const filteredStaffing = useMemo(() => {
    if (!hasOrgFilter) return staffing;
    return staffing.filter((s) => filters.orgs.has(s.org_name));
  }, [staffing, filters.orgs, hasOrgFilter]);

  const filteredKinds = useMemo(() => {
    if (!stats?.groups_by_kind) return [];
    if (!hasKindFilter) return stats.groups_by_kind;
    return stats.groups_by_kind.filter((k) => filters.kinds.has(k.kind));
  }, [stats, filters.kinds, hasKindFilter]);

  const filteredTimeline = useMemo(() => {
    if (!filters.dateFrom && !filters.dateTo) return timeline;
    return timeline.filter((t) => {
      if (filters.dateFrom && t.week < filters.dateFrom) return false;
      if (filters.dateTo && t.week > filters.dateTo) return false;
      return true;
    });
  }, [timeline, filters.dateFrom, filters.dateTo]);

  const filteredPipeline = useMemo((): PipelineData | null => {
    if (!pipeline) return null;
    if (!hasOrgFilter) return pipeline;
    const fo = filteredOrgData;
    if (fo.length === 0) return { planned: 0, arrived: 0, in_training: 0, completed: 0, attrition: 0 };
    return {
      planned: fo.reduce((s, o) => s + o.planned, 0),
      arrived: fo.reduce((s, o) => s + o.planned, 0),
      in_training: 0,
      completed: fo.reduce((s, o) => s + o.completed, 0),
      attrition: fo.reduce((s, o) => s + o.attrition, 0),
    };
  }, [pipeline, filteredOrgData, hasOrgFilter]);

  if (loading) {
    return (
      <div className="flex flex-col gap-6">
        <div>
          <h1>Аналітика</h1>
          <p className="text-sm text-muted-foreground">Завантаження даних...</p>
        </div>
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          {Array.from({ length: 4 }).map((_, i) => (
            <Skeleton key={i} className="h-24 rounded-xl" />
          ))}
        </div>
        <div className="grid gap-4 lg:grid-cols-2">
          {Array.from({ length: 4 }).map((_, i) => (
            <Skeleton key={i} className="h-64 rounded-xl" />
          ))}
        </div>
      </div>
    );
  }

  const fp = filteredPipeline;
  const completionRate =
    fp && fp.planned > 0 ? Math.round((fp.completed / fp.planned) * 100) : 0;
  const attritionRate =
    fp && fp.planned > 0 ? Math.round((fp.attrition / fp.planned) * 100) : 0;

  const filteredOrgCount = hasOrgFilter ? filters.orgs.size : (stats?.total_orgs ?? 0);
  const filteredGroupCount = hasKindFilter ? filteredKinds.reduce((s, k) => s + k.count, 0) : (stats?.total_groups ?? 0);
  const filteredDiscCount = filteredDiscData
    ? filteredDiscData.by_status.reduce((s, d) => s + d.count, 0)
    : (stats?.total_discrepancies ?? 0);

  return (
    <div className="flex flex-col gap-6">
      {/* Header */}
      <div className="flex items-start justify-between gap-3">
        <div>
          <h1>Аналітика</h1>
          <p className="text-sm text-muted-foreground">
            Зведена візуалізація стану підготовки, укомплектованості та розбіжностей
          </p>
        </div>
        {hasFilters && (
          <DropdownMenu onOpenChange={(open) => { if (open) loadRooms(); }}>
            <DropdownMenuTrigger asChild>
              <Button variant="outline" size="sm" className="gap-1.5 shrink-0">
                <Share2 className="h-3.5 w-3.5" />
                <span className="hidden sm:inline">Поділитись</span>
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onClick={() => {
                const url = buildShareUrl();
                navigator.clipboard.writeText(window.location.origin + url);
                toast.success("Посилання скопійовано");
              }}>
                <Copy className="mr-2 h-4 w-4" />
                Копіювати посилання
              </DropdownMenuItem>
              <DropdownMenuSub>
                <DropdownMenuSubTrigger>
                  <MessageSquare className="mr-2 h-4 w-4" />
                  Надіслати в чат
                </DropdownMenuSubTrigger>
                <DropdownMenuSubContent>
                  {chatRooms.length === 0 ? (
                    <DropdownMenuItem disabled>Завантаження...</DropdownMenuItem>
                  ) : chatRooms.map((room) => (
                    <DropdownMenuItem
                      key={room.id}
                      onClick={() => {
                        const url = buildShareUrl();
                        const parts = [
                          filters.orgs.size && `підрозділи: ${[...filters.orgs].join(", ")}`,
                          filters.kinds.size && `вид: ${[...filters.kinds].join(", ")}`,
                          filters.dateFrom && `з ${filters.dateFrom}`,
                          filters.dateTo && `по ${filters.dateTo}`,
                        ].filter(Boolean).join(", ");
                        const text = parts ? `${url} (${parts})` : url;
                        navigate(`/chat?room=${room.id}&prefill=${encodeURIComponent(text)}`);
                      }}
                    >
                      {room.emoji ? `${room.emoji} ` : ""}{room.name}
                    </DropdownMenuItem>
                  ))}
                </DropdownMenuSubContent>
              </DropdownMenuSub>
            </DropdownMenuContent>
          </DropdownMenu>
        )}
      </div>

      {/* Filters */}
      <DashboardFilters
        filters={filters}
        onChange={setFilters}
        availableOrgs={availableOrgs}
      />

      {/* Summary cards */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <StatCard
          title="Підрозділів"
          value={filteredOrgCount}
          icon={Building2}
          color={CHART_COLORS.gold}
        />
        <StatCard
          title="Груп підготовки"
          value={filteredGroupCount}
          icon={Users}
          color={CHART_COLORS.blue}
          subtitle={`${completionRate}% завершення`}
        />
        <StatCard
          title="Відкритих розбіжностей"
          value={filteredDiscCount}
          icon={AlertTriangle}
          color={CHART_COLORS.red}
        />
        <StatCard
          title="Рівень вибуття"
          value={`${attritionRate}%`}
          icon={TrendingUp}
          color={attritionRate > 15 ? CHART_COLORS.red : CHART_COLORS.green}
          subtitle={fp ? `${fp.attrition} з ${fp.planned}` : undefined}
        />
      </div>

      {/* Pipeline + Kind breakdown */}
      <div className="grid gap-4 lg:grid-cols-5">
        <ChartSection title="Воронка підготовки" icon={Target} className="lg:col-span-3">
          {fp ? <PipelineChart data={fp} /> : <EmptyState />}
        </ChartSection>
        <ChartSection title="За видами підготовки" icon={PieIcon} className="lg:col-span-2">
          {filteredKinds.length ? (
            <KindDonut data={filteredKinds} />
          ) : (
            <EmptyState />
          )}
        </ChartSection>
      </div>

      {/* Timeline */}
      <ChartSection title="Динаміка по тижнях" icon={Calendar}>
        {filteredTimeline.length > 0 ? <TimelineChart data={filteredTimeline} /> : <EmptyState text="Недостатньо даних для графіку динаміки" />}
      </ChartSection>

      {/* Org breakdown + Discrepancies */}
      <div className="grid gap-4 lg:grid-cols-2">
        <ChartSection title="Підготовка по підрозділах" icon={BarChart3}>
          {filteredOrgData.length > 0 ? <OrgBarChart data={filteredOrgData} /> : <EmptyState />}
        </ChartSection>
        <ChartSection title="Розбіжності" icon={AlertTriangle}>
          {filteredDiscData ? (
            <div className="flex flex-col gap-4">
              <DiscStatusDonut data={filteredDiscData.by_status} />
              {filteredDiscData.by_org.length > 0 && <DiscOrgChart data={filteredDiscData.by_org} />}
            </div>
          ) : (
            <EmptyState />
          )}
        </ChartSection>
      </div>

      {/* Staffing */}
      <ChartSection title="Укомплектованість" icon={Shield}>
        <StaffingChart data={filteredStaffing} />
      </ChartSection>
    </div>
  );
}

function EmptyState({ text }: { text?: string }) {
  return (
    <div className="flex items-center justify-center py-8 text-muted-foreground">
      <p className="text-sm italic">{text || "Немає даних"}</p>
    </div>
  );
}
