import { useEffect, useState, useMemo, useCallback } from "react";
import { useSearchParams, useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { api } from "@/api/client";
import type { ChatRoom } from "@/api/types";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
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
  BarChart3,
  PieChart as PieIcon,
  Calendar,
  Shield,
  RotateCcw,
  Share2,
  MessageSquare,
  Copy,
  Maximize2,
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
// Visual helpers: count-up, sparkline, ring gauge
// ---------------------------------------------------------------------------

function useCountUp(target: number, ms = 900) {
  const [v, setV] = useState(0);
  useEffect(() => {
    let raf = 0;
    const t0 = performance.now();
    const tick = (now: number) => {
      const p = Math.min(1, (now - t0) / ms);
      const e = 1 - Math.pow(1 - p, 3);
      setV(Math.round(target * e));
      if (p < 1) raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [target, ms]);
  return v;
}

function Spark({ values, color, id }: { values: number[]; color: string; id: string }) {
  if (values.length < 2) return null;
  const W = 120, H = 36, pad = 3;
  const min = Math.min(...values), max = Math.max(...values);
  const rng = max - min || 1;
  const pts = values.map((v, i) => [pad + (i / (values.length - 1)) * (W - pad * 2), H - pad - ((v - min) / rng) * (H - pad * 2)]);
  const line = pts.map((p, i) => `${i ? "L" : "M"}${p[0].toFixed(1)},${p[1].toFixed(1)}`).join(" ");
  const area = `${line} L${pts[pts.length - 1][0]},${H} L${pts[0][0]},${H} Z`;
  const last = pts[pts.length - 1];
  return (
    <svg width={W} height={H} viewBox={`0 0 ${W} ${H}`} className="shrink-0" aria-hidden="true">
      <defs>
        <linearGradient id={`sp-${id}`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor={color} stopOpacity={0.35} />
          <stop offset="100%" stopColor={color} stopOpacity={0} />
        </linearGradient>
      </defs>
      <path d={area} fill={`url(#sp-${id})`} />
      <path d={line} fill="none" stroke={color} strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" />
      <circle cx={last[0]} cy={last[1]} r={3.2} fill={color} />
    </svg>
  );
}

function Ring({ pct, color, size = 132, stroke = 12, children }: { pct: number; color: string; size?: number; stroke?: number; children?: React.ReactNode }) {
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  const [shown, setShown] = useState(0);
  useEffect(() => {
    const t = setTimeout(() => setShown(Math.max(0, Math.min(100, pct))), 60);
    return () => clearTimeout(t);
  }, [pct]);
  return (
    <div className="relative shrink-0" style={{ width: size, height: size }}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} className="-rotate-90">
        <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke="var(--muted)" strokeWidth={stroke} />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke={color}
          strokeWidth={stroke}
          strokeLinecap="round"
          strokeDasharray={c}
          strokeDashoffset={c * (1 - shown / 100)}
          style={{ transition: "stroke-dashoffset 1s cubic-bezier(.2,.8,.2,1)", filter: `drop-shadow(0 0 6px ${color}66)` }}
        />
      </svg>
      <div className="absolute inset-0 flex flex-col items-center justify-center">{children}</div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Stat card
// ---------------------------------------------------------------------------

function StatCard({
  title,
  value,
  suffix,
  icon: Icon,
  color,
  subtitle,
  trend,
  delta,
  goodWhenUp = true,
  id,
}: {
  title: string;
  value: number;
  suffix?: string;
  icon: React.ElementType;
  color: string;
  subtitle?: string;
  trend?: number[];
  delta?: number | null;
  goodWhenUp?: boolean;
  id: string;
}) {
  const shown = useCountUp(value);
  const up = (delta ?? 0) > 0;
  const good = delta ? up === goodWhenUp : true;
  return (
    <Card
      className="card-animate card-hover relative overflow-hidden"
      style={{ background: `linear-gradient(155deg, ${color}18 0%, transparent 55%), var(--card)` }}
    >
      <span className="absolute inset-x-0 top-0 h-[3px]" style={{ background: `linear-gradient(90deg, ${color}, ${color}00)` }} />
      <span className="pointer-events-none absolute -right-8 -top-8 h-32 w-32 rounded-full blur-2xl" style={{ background: `${color}22` }} />
      <CardContent className="relative flex flex-col gap-3 p-5">
        <div className="flex items-center gap-2.5">
          <div
            className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl"
            style={{ background: `${color}22`, border: `1px solid ${color}40`, boxShadow: `0 0 18px ${color}25` }}
          >
            <Icon className="h-5 w-5" style={{ color }} />
          </div>
          <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">{title}</p>
        </div>
        <div className="flex items-end justify-between gap-3">
          <div className="min-w-0">
            <p className="font-heading text-5xl leading-none font-bold tabular-nums" style={{ color }}>
              {shown.toLocaleString("uk-UA")}
              {suffix && <span className="ml-1 text-2xl opacity-80">{suffix}</span>}
            </p>
            <div className="mt-2 flex items-center gap-2 text-xs">
              {delta != null && delta !== 0 && (
                <span
                  className="inline-flex items-center rounded-full px-2 py-0.5 font-bold tabular-nums"
                  style={{
                    background: good ? `${CHART_COLORS.green}22` : `${CHART_COLORS.red}22`,
                    color: good ? CHART_COLORS.green : CHART_COLORS.red,
                  }}
                  title="Зміна проти попереднього тижня"
                >
                  {up ? "▲" : "▼"} {Math.abs(delta)}
                </span>
              )}
              {subtitle && <span className="text-muted-foreground">{subtitle}</span>}
            </div>
          </div>
          {trend && <Spark values={trend} color={color} id={id} />}
        </div>
      </CardContent>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// Pipeline funnel
// ---------------------------------------------------------------------------

function PipelineChart({ data, partial = false }: { data: PipelineData; partial?: boolean }) {
  const stages = [
    { name: "Заплановано", value: data.planned, color: CHART_COLORS.blue },
    { name: "Прибуло", value: data.arrived, color: CHART_COLORS.teal },
    { name: "Навчається", value: data.in_training, color: CHART_COLORS.gold },
    { name: "Завершило", value: data.completed, color: CHART_COLORS.green },
    { name: "Вибуло", value: data.attrition, color: CHART_COLORS.red },
  ].filter((st) => !partial || (st.name !== "Прибуло" && st.name !== "Навчається"));
  const maxVal = Math.max(...stages.map((s) => s.value), 1);
  const [go, setGo] = useState(false);
  useEffect(() => {
    const t = setTimeout(() => setGo(true), 60);
    return () => clearTimeout(t);
  }, []);

  return (
    <div className="flex flex-col gap-4">
      {stages.map((stage, i) => {
        const pct = (stage.value / maxVal) * 100;
        const prev = !partial && i > 0 && i < 4 ? stages[i - 1].value : null;
        const conv = prev && prev > 0 ? Math.round((stage.value / prev) * 100) : null;
        const ofPlan = data.planned > 0 ? Math.round((stage.value / data.planned) * 100) : 0;
        return (
          <div key={i} className="flex items-center gap-2 sm:gap-4">
            <span className="w-20 shrink-0 text-right text-xs font-medium text-muted-foreground sm:w-28 sm:text-sm">{stage.name}</span>
            <div className="relative h-10 flex-1 overflow-hidden rounded-xl sm:h-12" style={{ background: "var(--muted)" }}>
              <div
                className="flex h-full items-center rounded-xl px-4 text-lg font-bold tabular-nums"
                style={{
                  width: go ? `${Math.max(pct, 4)}%` : "0%",
                  background: `linear-gradient(90deg, ${stage.color}, ${stage.color}b0)`,
                  color: "#14140c",
                  boxShadow: `0 0 22px ${stage.color}55, inset 0 1px 0 rgba(255,255,255,0.25)`,
                  transition: `width 900ms cubic-bezier(.2,.8,.2,1) ${i * 90}ms`,
                }}
              >
                {stage.value.toLocaleString("uk-UA")}
              </div>
            </div>
            <div className="hidden w-24 shrink-0 text-xs leading-tight sm:block">
              <p className="font-bold tabular-nums" style={{ color: stage.color }}>{ofPlan}% плану</p>
              {conv != null && <p className="text-muted-foreground tabular-nums">{conv}% від попер.</p>}
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
    <div className="flex flex-col items-center gap-5 sm:flex-row lg:flex-col xl:flex-row">
      <div className="relative shrink-0" style={{ width: 220, height: 220 }}>
        <PieChart width={220} height={220}>
          <Pie
            data={chartData}
            cx="50%"
            cy="50%"
            innerRadius={72}
            outerRadius={104}
            paddingAngle={4}
            cornerRadius={6}
            dataKey="value"
            strokeWidth={0}
            isAnimationActive={false}
          >
            {chartData.map((d, i) => (
              <Cell key={i} fill={d.color} style={{ filter: `drop-shadow(0 0 8px ${d.color}66)` }} />
            ))}
          </Pie>
          <Tooltip content={<ChartTooltip />} />
        </PieChart>
        <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center">
          <span className="font-heading text-5xl font-bold tabular-nums">{total}</span>
          <span className="text-xs uppercase tracking-wider text-muted-foreground">груп</span>
        </div>
      </div>
      <div className="flex w-full min-w-0 flex-1 flex-col gap-3">
        {chartData.map((d, i) => {
          const pct = total > 0 ? Math.round((d.value / total) * 100) : 0;
          return (
            <div key={i} className="flex flex-col gap-1">
              <div className="flex items-center gap-2 text-sm">
                <span className="h-3 w-3 shrink-0 rounded-full" style={{ background: d.color, boxShadow: `0 0 8px ${d.color}` }} />
                <span className="flex-1 font-medium">{d.name}</span>
                <span className="font-bold tabular-nums">{d.value}</span>
                <span className="w-10 text-right text-xs text-muted-foreground tabular-nums">{pct}%</span>
              </div>
              <div className="h-1.5 overflow-hidden rounded-full" style={{ background: "var(--muted)" }}>
                <div className="h-full rounded-full" style={{ width: `${pct}%`, background: d.color }} />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Org bar chart
// ---------------------------------------------------------------------------

function OrgBarChart({ data, full = false }: { data: OrgData[]; full?: boolean }) {
  // Рейтинг замість графіка: висота не залежить від кількості підрозділів (список прокручується).
  const sorted = [...data].sort((a, b) => b.planned - a.planned);
  const LIMIT = 6;
  const rows = full ? sorted : sorted.slice(0, LIMIT);
  const maxPlanned = Math.max(...rows.map((r) => r.planned), 1);
  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
        <span className="flex items-center gap-1.5"><i className="h-2.5 w-2.5 rounded-sm" style={{ background: CHART_COLORS.green }} />Завершило</span>
        <span className="flex items-center gap-1.5"><i className="h-2.5 w-2.5 rounded-sm" style={{ background: CHART_COLORS.red }} />Вибуло</span>
        <span className="flex items-center gap-1.5"><i className="h-2.5 w-2.5 rounded-sm" style={{ background: CHART_COLORS.blue }} />Решта плану</span>
      </div>
      <ul className="flex flex-col gap-2.5">
        {rows.map((r) => {
          const done = r.planned > 0 ? Math.round((r.completed / r.planned) * 100) : 0;
          const w = (r.planned / maxPlanned) * 100;
          const pc = r.planned > 0 ? (r.completed / r.planned) * 100 : 0;
          const pa = r.planned > 0 ? (r.attrition / r.planned) * 100 : 0;
          return (
            <li key={r.org_name} className="flex items-center gap-3" title={`${r.org_name}: план ${r.planned}, завершило ${r.completed}, вибуло ${r.attrition}`}>
              <span className="w-24 shrink-0 truncate text-sm font-semibold">{r.org_name}</span>
              <div className="h-3 flex-1 rounded-full" style={{ background: "var(--muted)" }}>
                <div
                  className="flex h-full overflow-hidden rounded-full"
                  style={{ width: `${Math.max(w, 3)}%`, background: CHART_COLORS.blue, boxShadow: `0 0 10px ${CHART_COLORS.blue}40` }}
                >
                  <span style={{ width: `${pc}%`, background: CHART_COLORS.green }} />
                  <span style={{ width: `${pa}%`, background: CHART_COLORS.red }} />
                </div>
              </div>
              <span className="w-16 shrink-0 text-right text-xs tabular-nums text-muted-foreground">{r.completed}/{r.planned}</span>
              <span className="w-10 shrink-0 text-right text-sm font-bold tabular-nums" style={{ color: done >= 70 ? CHART_COLORS.green : done >= 30 ? CHART_COLORS.gold : "var(--muted-foreground)" }}>{done}%</span>
            </li>
          );
        })}
      </ul>
      {!full && sorted.length > LIMIT && (
        <p className="text-xs text-muted-foreground">Ще {sorted.length - LIMIT} — натисніть на картку, щоб побачити всі.</p>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Timeline area chart
// ---------------------------------------------------------------------------

function TimelineChart({ data, full = false }: { data: TimelinePoint[]; full?: boolean }) {
  const formatted = data.map((d) => ({
    ...d,
    label: new Date(d.week).toLocaleDateString("uk-UA", { day: "numeric", month: "short" }),
  }));

  return (
    <ResponsiveContainer width="100%" height={full ? 440 : 230}>
      <AreaChart data={formatted} margin={{ left: 0, right: 16, top: 8, bottom: 8 }}>
        <defs>
          <linearGradient id="gradPlanned" x1="0" y1="0" x2="0" y2="1">
            <stop offset="5%" stopColor={CHART_COLORS.blue} stopOpacity={0.5} />
            <stop offset="95%" stopColor={CHART_COLORS.blue} stopOpacity={0.02} />
          </linearGradient>
          <linearGradient id="gradStarted" x1="0" y1="0" x2="0" y2="1">
            <stop offset="5%" stopColor={CHART_COLORS.gold} stopOpacity={0.5} />
            <stop offset="95%" stopColor={CHART_COLORS.gold} stopOpacity={0.02} />
          </linearGradient>
          <linearGradient id="gradCompleted" x1="0" y1="0" x2="0" y2="1">
            <stop offset="5%" stopColor={CHART_COLORS.green} stopOpacity={0.5} />
            <stop offset="95%" stopColor={CHART_COLORS.green} stopOpacity={0.02} />
          </linearGradient>
        </defs>
        <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" opacity={0.3} />
        <XAxis dataKey="label" tick={{ fill: "var(--muted-foreground)", fontSize: 10 }} />
        <YAxis allowDecimals={false} tick={{ fill: "var(--muted-foreground)", fontSize: 12 }} />
        <Tooltip content={<ChartTooltip />} />
        <Legend
          wrapperStyle={{ fontSize: 13 }}
          formatter={(v: string) => <span style={{ color: "var(--foreground)" }}>{v}</span>}
        />
        <Area isAnimationActive={false}
          type="monotone"
          dataKey="planned"
          name="Заплановано"
          stroke={CHART_COLORS.blue}
          fill="url(#gradPlanned)"
          strokeWidth={3}
          dot={{ r: 3.5, strokeWidth: 0 }}
          activeDot={{ r: 6 }}
        />
        <Area isAnimationActive={false}
          type="monotone"
          dataKey="started"
          name="Розпочато"
          stroke={CHART_COLORS.gold}
          fill="url(#gradStarted)"
          strokeWidth={3}
          dot={{ r: 3.5, strokeWidth: 0 }}
          activeDot={{ r: 6 }}
        />
        <Area isAnimationActive={false}
          type="monotone"
          dataKey="completed"
          name="Завершено"
          stroke={CHART_COLORS.green}
          fill="url(#gradCompleted)"
          strokeWidth={3}
          dot={{ r: 3.5, strokeWidth: 0 }}
          activeDot={{ r: 6 }}
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
  const shown = chartData.filter((d) => d.value > 0);

  return (
    <div className="flex flex-col items-center gap-5 sm:flex-row">
      <div className="relative shrink-0" style={{ width: 190, height: 190 }}>
        <PieChart width={190} height={190}>
          <Pie
            data={shown.length ? shown : [{ name: "—", value: 1, color: "var(--muted)" }]}
            cx="50%"
            cy="50%"
            innerRadius={62}
            outerRadius={90}
            paddingAngle={shown.length > 1 ? 4 : 0}
            cornerRadius={6}
            dataKey="value"
            strokeWidth={0}
            isAnimationActive={false}
          >
            {(shown.length ? shown : [{ color: "var(--muted)" }]).map((d: any, i: number) => (
              <Cell key={i} fill={d.color} style={{ filter: shown.length ? `drop-shadow(0 0 8px ${d.color}66)` : undefined }} />
            ))}
          </Pie>
          <Tooltip content={<ChartTooltip />} />
        </PieChart>
        <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center">
          <span className="font-heading text-4xl font-bold tabular-nums">{total}</span>
          <span className="text-[11px] uppercase tracking-wider text-muted-foreground">всього</span>
        </div>
      </div>
      <div className="grid w-full flex-1 grid-cols-2 gap-2.5">
        {chartData.map((d, i) => (
          <div
            key={i}
            className="rounded-xl px-3 py-2"
            style={{ background: `${d.color}14`, border: `1px solid ${d.color}30` }}
          >
            <p className="font-heading text-2xl font-bold tabular-nums leading-none" style={{ color: d.color }}>{d.value}</p>
            <p className="mt-1 text-xs text-muted-foreground">{d.name}</p>
          </div>
        ))}
      </div>
    </div>
  );
}

function DiscOrgChart({ data }: { data: { org_name: string; open: number; resolved: number; accepted: number }[] }) {
  return (
    <ResponsiveContainer width="100%" height={280}>
      <BarChart data={data} margin={{ left: 0, right: 16, top: 8, bottom: 8 }}>
        <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" opacity={0.3} />
        <XAxis dataKey="org_name" tick={{ fill: "var(--muted-foreground)", fontSize: 10 }} />
        <YAxis allowDecimals={false} tick={{ fill: "var(--muted-foreground)", fontSize: 12 }} />
        <Tooltip content={<ChartTooltip />} cursor={{ fill: "rgba(255,255,255,0.05)" }} />
        <Legend
          wrapperStyle={{ fontSize: 13 }}
          formatter={(v: string) => <span style={{ color: "var(--foreground)" }}>{v}</span>}
        />
        <Bar isAnimationActive={false} dataKey="open" name="Відкриті" fill={CHART_COLORS.red} stackId="a" radius={[0, 0, 0, 0]} />
        <Bar isAnimationActive={false} dataKey="resolved" name="Вирішені" fill={CHART_COLORS.green} stackId="a" radius={[0, 0, 0, 0]} />
        <Bar isAnimationActive={false} dataKey="accepted" name="Прийняті" fill={CHART_COLORS.blue} stackId="a" radius={[8, 8, 0, 0]} />
      </BarChart>
    </ResponsiveContainer>
  );
}

// ---------------------------------------------------------------------------
// Staffing gauge
// ---------------------------------------------------------------------------

function StaffingChart({ data, full = false }: { data: StaffingRow[]; full?: boolean }) {
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

  const totA = grouped.reduce((s, g) => s + g.authorized, 0);
  const totS = grouped.reduce((s, g) => s + g.assigned, 0);
  const totPct = totA > 0 ? Math.round((totS / totA) * 100) : 0;
  const colorOf = (p: number) => (p >= 80 ? CHART_COLORS.green : p >= 50 ? CHART_COLORS.gold : CHART_COLORS.red);

  return (
    <div className="grid grid-cols-2 gap-3 sm:gap-4 lg:grid-cols-[1.2fr_repeat(4,1fr)]">
      <div
        className="flex flex-col items-center justify-center gap-2 rounded-2xl p-4 col-span-2 lg:col-span-1"
        style={{ background: `${colorOf(totPct)}12`, border: `1px solid ${colorOf(totPct)}35` }}
      >
        <Ring pct={totPct} color={colorOf(totPct)} size={150} stroke={14}>
          <span className="font-heading text-4xl font-bold tabular-nums" style={{ color: colorOf(totPct) }}>{totPct}%</span>
          <span className="text-[11px] uppercase tracking-wider text-muted-foreground">разом</span>
        </Ring>
        <p className="text-xs text-muted-foreground tabular-nums">{totS} з {totA} штатних посад</p>
      </div>
      {(full ? grouped : [...grouped].sort((a, b) => a.fill_pct - b.fill_pct).slice(0, 4)).map((g, i) => {
        const c = colorOf(g.fill_pct);
        return (
          <div
            key={i}
            className="card-hover flex flex-col items-center gap-2 rounded-2xl p-4"
            style={{ background: "var(--muted)", border: "1px solid var(--border)" }}
          >
            <Ring pct={g.fill_pct} color={c} size={104} stroke={10}>
              <span className="font-heading text-2xl font-bold tabular-nums" style={{ color: c }}>{g.fill_pct}%</span>
            </Ring>
            <p className="max-w-full truncate text-sm font-semibold">{g.org_name}</p>
            <p className="text-xs text-muted-foreground tabular-nums">{g.assigned} / {g.authorized}</p>
          </div>
        );
      })}
      {!full && grouped.length > 4 && (
        <p className="col-span-full text-xs text-muted-foreground">
          Показано 4 найнижчих з {grouped.length}. Натисніть на картку, щоб побачити всі підрозділи.
        </p>
      )}
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
  accent = CHART_COLORS.gold,
  expandable = false,
}: {
  title: string;
  icon: React.ElementType;
  /** Функція отримує `full`: false — компактний огляд у картці, true — повний вміст у модалці. */
  children: React.ReactNode | ((full: boolean) => React.ReactNode);
  className?: string;
  accent?: string;
  expandable?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const render = (full: boolean) => (typeof children === "function" ? children(full) : children);
  const headIcon = (
    <div
      className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl"
      style={{ background: `${accent}1f`, border: `1px solid ${accent}40`, color: accent }}
    >
      <Icon className="h-4.5 w-4.5" />
    </div>
  );
  return (
    <>
      <Card
        className={`card-animate relative overflow-hidden ${expandable ? "card-hover cursor-pointer" : ""} ${className ?? ""}`}
        onClick={expandable ? () => setOpen(true) : undefined}
        onKeyDown={expandable ? (e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); setOpen(true); } } : undefined}
        role={expandable ? "button" : undefined}
        tabIndex={expandable ? 0 : undefined}
        aria-label={expandable ? `${title} — відкрити докладно` : undefined}
      >
        <span className="absolute inset-x-0 top-0 h-[3px]" style={{ background: `linear-gradient(90deg, ${accent}, ${accent}00 70%)` }} />
        <CardHeader className="pb-3 pt-5">
          <CardTitle className="flex items-center gap-3 text-base">
            {headIcon}
            <span className="flex-1">{title}</span>
            {expandable && (
              <span className="flex items-center gap-1.5 text-xs font-normal text-muted-foreground">
                <span className="hidden sm:inline">Докладно</span>
                <Maximize2 className="h-4 w-4" />
              </span>
            )}
          </CardTitle>
        </CardHeader>
        <CardContent className="pb-5">{render(false)}</CardContent>
      </Card>
      {expandable && (
        <Dialog open={open} onOpenChange={setOpen}>
          <DialogContent className="max-h-[90vh] w-[96vw] overflow-y-auto sm:max-w-5xl">
            <DialogHeader>
              <DialogTitle className="flex items-center gap-3 text-lg">
                {headIcon}
                {title}
              </DialogTitle>
            </DialogHeader>
            <div className="min-h-0">{render(true)}</div>
          </DialogContent>
        </Dialog>
      )}
    </>
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

  const weekDelta = (key: "started" | "attrition" | "completed" | "planned"): number | null => {
    if (filteredTimeline.length < 2) return null;
    const last = filteredTimeline[filteredTimeline.length - 1][key];
    const prev = filteredTimeline[filteredTimeline.length - 2][key];
    return last - prev;
  };

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
      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <StatCard
          id="orgs"
          title="Підрозділів"
          value={filteredOrgCount}
          icon={Building2}
          color={CHART_COLORS.gold}
          subtitle="у складі системи"
        />
        <StatCard
          id="groups"
          title="Груп підготовки"
          value={filteredGroupCount}
          icon={Users}
          color={CHART_COLORS.blue}
          subtitle={`${completionRate}% завершення`}
          trend={filteredTimeline.map((t) => t.started)}
          delta={weekDelta("started")}
        />
        <StatCard
          id="disc"
          title="Відкритих розбіжностей"
          value={filteredDiscCount}
          icon={AlertTriangle}
          color={filteredDiscCount > 0 ? CHART_COLORS.red : CHART_COLORS.green}
          subtitle={filteredDiscCount > 0 ? "потребують уваги" : "все узгоджено"}
          goodWhenUp={false}
        />
        <StatCard
          id="attr"
          title="Рівень вибуття"
          value={attritionRate}
          suffix="%"
          icon={TrendingUp}
          color={attritionRate > 15 ? CHART_COLORS.red : CHART_COLORS.green}
          subtitle={fp ? `${fp.attrition} з ${fp.planned}` : undefined}
          trend={filteredTimeline.map((t) => t.attrition)}
          delta={weekDelta("attrition")}
          goodWhenUp={false}
        />
      </div>

      {/* Pipeline + Kind breakdown */}
      <div className="grid gap-4 lg:grid-cols-5">
        <ChartSection title="Воронка підготовки" icon={Target} accent={CHART_COLORS.blue} className="lg:col-span-3">
          {fp ? (
            <div className="flex flex-col gap-3">
              <PipelineChart data={fp} partial={hasOrgFilter} />
              {hasOrgFilter && (
                <p className="text-xs text-muted-foreground">
                  Для вибраних підрозділів етапи "Прибуло" і "Навчається" не показуються: розбивка по підрозділах їх не містить.
                </p>
              )}
            </div>
          ) : <EmptyState />}
        </ChartSection>
        <ChartSection title="За видами підготовки" icon={PieIcon} accent={CHART_COLORS.teal} className="lg:col-span-2">
          {filteredKinds.length ? (
            <KindDonut data={filteredKinds} />
          ) : (
            <EmptyState />
          )}
        </ChartSection>
      </div>

      {/* Timeline */}
      <ChartSection title="Динаміка по тижнях" icon={Calendar} accent={CHART_COLORS.purple} expandable>
        {(full) => filteredTimeline.length > 0 ? <TimelineChart data={filteredTimeline} full={full} /> : <EmptyState text="Недостатньо даних для графіку динаміки" />}
      </ChartSection>

      {/* Org breakdown + Discrepancies */}
      <div className="grid items-start gap-4 lg:grid-cols-2">
        <ChartSection title="Підготовка по підрозділах" icon={BarChart3} accent={CHART_COLORS.green} expandable>
          {(full) => filteredOrgData.length > 0 ? <OrgBarChart data={filteredOrgData} full={full} /> : <EmptyState />}
        </ChartSection>
        <ChartSection title="Розбіжності" icon={AlertTriangle} accent={CHART_COLORS.red} expandable>
          {(full) => filteredDiscData ? (
            <div className="flex flex-col gap-4">
              {filteredDiscData.by_status.reduce((s, d) => s + d.count, 0) === 0 ? (
                <div className="flex flex-col items-center gap-3 py-10 text-center">
                  <div
                    className="flex h-16 w-16 items-center justify-center rounded-full"
                    style={{ background: `${CHART_COLORS.green}1f`, border: `1px solid ${CHART_COLORS.green}55`, boxShadow: `0 0 24px ${CHART_COLORS.green}33` }}
                  >
                    <Shield className="h-7 w-7" style={{ color: CHART_COLORS.green }} />
                  </div>
                  <p className="font-heading text-xl font-bold" style={{ color: CHART_COLORS.green }}>Розбіжностей немає</p>
                  <p className="max-w-xs text-sm text-muted-foreground">Дані підрозділів і вищих рівнів збігаються.</p>
                </div>
              ) : (
                <>
                  <DiscStatusDonut data={filteredDiscData.by_status} />
                  {full && filteredDiscData.by_org.some((o) => o.open + o.resolved + o.accepted > 0) && (
                    <DiscOrgChart data={filteredDiscData.by_org.filter((o) => o.open + o.resolved + o.accepted > 0)} />
                  )}
                </>
              )}
            </div>
          ) : (
            <EmptyState />
          )}
        </ChartSection>
      </div>

      {/* Staffing */}
      <ChartSection title="Укомплектованість" icon={Shield} accent={CHART_COLORS.orange} expandable>
        {(full) => <StaffingChart data={filteredStaffing} full={full} />}
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
