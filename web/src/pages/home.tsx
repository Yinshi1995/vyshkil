import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useAuth } from "@/context/auth";
import { api } from "@/api/client";
import type { DashboardStats } from "@/api/types";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import {
  Building2,
  Users,
  FileText,
  AlertTriangle,
  FileSpreadsheet,
  Upload,
  ArrowRight,
  Activity,
  BarChart3,
  Clock,
} from "lucide-react";
import { sourceTypeLabel } from "@/lib/labels";

const KIND_LABELS: Record<string, string> = {
  bzvp: "БЗВП",
  special: "Фахова",
  adaptation: "Адаптація",
  internship: "Стажування",
};

const KIND_COLORS: Record<string, string> = {
  bzvp: "#8fa565",
  special: "#5B9BD5",
  adaptation: "#c9a84c",
  internship: "#c97a6e",
};

const METRIC_LABELS: Record<string, string> = {
  arrived_count: "Прибуло",
  planned_count: "Заплановано",
  current_count: "Наявних",
  dropped_count: "Вибуло",
  completed_count: "Завершило",
  total_count: "Всього",
};

function StatCard({
  title,
  value,
  icon: Icon,
  href,
  accent,
  tint,
}: {
  title: string;
  value: number | null;
  icon: React.ElementType;
  href: string;
  accent?: boolean;
  tint?: string;
}) {
  const iconColor = accent ? "var(--destructive)" : tint ?? "var(--primary)";
  return (
    <Link to={href} className="group block">
      <Card className="transition-all duration-300 group-hover:scale-[1.02]">
        <CardContent className="flex items-center gap-4 p-5">
          <div
            className="icon-box"
            style={{
              borderColor: iconColor,
              color: iconColor,
              background: `radial-gradient(circle at 30% 30%, color-mix(in srgb, ${iconColor} 10%, transparent), transparent 70%)`,
            }}
          >
            <Icon className="h-5 w-5" />
          </div>
          <div className="flex-1">
            <p className="eyebrow-label">{title}</p>
            {value !== null ? (
              <p className="stat-value" style={{ fontSize: "28px", color: iconColor }}>
                {value.toLocaleString("uk-UA")}
              </p>
            ) : (
              <Skeleton className="h-8 w-14" />
            )}
          </div>
          <ArrowRight className="h-4 w-4 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100" />
        </CardContent>
      </Card>
    </Link>
  );
}

function KindBreakdown({ data }: { data: DashboardStats["groups_by_kind"] }) {
  if (data.length === 0) return null;
  const total = data.reduce((s, d) => s + d.count, 0);

  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="flex items-center gap-2 text-sm">
          <BarChart3 className="h-4 w-4 text-primary" />
          Групи за видами підготовки
        </CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        {data.map((k) => {
          const pct = total > 0 ? Math.round((k.count / total) * 100) : 0;
          const barColor = KIND_COLORS[k.kind] ?? "var(--primary)";
          return (
            <div key={k.kind} className="flex flex-col gap-1">
              <div className="flex items-center justify-between">
                <span className="text-sm">{KIND_LABELS[k.kind] ?? k.kind}</span>
                <span className="stat-value" style={{ fontSize: "16px", color: barColor }}>
                  {k.count}
                </span>
              </div>
              <div
                className="h-1.5 w-full overflow-hidden rounded-full"
                style={{ background: `color-mix(in srgb, ${barColor} 12%, transparent)` }}
              >
                <div
                  className="h-full rounded-full transition-all duration-700"
                  style={{
                    width: `${pct}%`,
                    background: barColor,
                  }}
                />
              </div>
            </div>
          );
        })}
      </CardContent>
    </Card>
  );
}

function RecentActivity({ stats }: { stats: DashboardStats }) {
  const items = [
    ...stats.recent_submissions.map((s) => ({
      id: `sub-${s.id}`,
      icon: Upload,
      label: s.org_label,
      detail: sourceTypeLabel(s.source_type),
      time: s.updated_at,
      type: "submission" as const,
    })),
    ...stats.recent_discrepancies.map((d) => ({
      id: `disc-${d.id}`,
      icon: AlertTriangle,
      label: d.org_label,
      detail: METRIC_LABELS[d.metric_label] ?? d.metric_label,
      time: d.created_at,
      type: "discrepancy" as const,
    })),
  ].sort((a, b) => b.time.localeCompare(a.time)).slice(0, 8);

  if (items.length === 0) {
    return (
      <Card>
        <CardContent className="flex flex-col items-center gap-2 py-8 text-muted-foreground">
          <Activity className="h-8 w-8 opacity-40" />
          <p className="text-sm">Поки що немає активності</p>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="flex items-center gap-2 text-sm">
          <Clock className="h-4 w-4 text-primary" />
          Остання активність
        </CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-0">
        {items.map((item, i) => {
          const Icon = item.icon;
          return (
            <div key={item.id}>
              <div className="flex items-center gap-3 py-2.5">
                <div
                  className="flex h-8 w-8 shrink-0 items-center justify-center rounded"
                  style={{
                    background:
                      item.type === "discrepancy"
                        ? "color-mix(in srgb, var(--destructive) 12%, transparent)"
                        : "color-mix(in srgb, var(--primary) 8%, transparent)",
                    color:
                      item.type === "discrepancy"
                        ? "var(--destructive)"
                        : "var(--primary)",
                  }}
                >
                  <Icon className="h-3.5 w-3.5" />
                </div>
                <div className="min-w-0 flex-1">
                  <p className="truncate text-sm font-medium">{item.label}</p>
                  <p className="truncate text-xs text-muted-foreground">{item.detail}</p>
                </div>
                <span className="shrink-0 text-xs text-muted-foreground">{item.time}</span>
              </div>
              {i < items.length - 1 && <Separator />}
            </div>
          );
        })}
      </CardContent>
    </Card>
  );
}

function QuickActions() {
  const actions = [
    { href: "/training", icon: FileText, label: "Підготовка" },
    { href: "/data", icon: Upload, label: "Робочий стіл даних" },
    { href: "/documents", icon: FileSpreadsheet, label: "Документи" },
    { href: "/discrepancies", icon: AlertTriangle, label: "Розбіжності" },
  ];

  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="flex items-center gap-2 text-sm">
          <Activity className="h-4 w-4 text-primary" />
          Швидкий доступ
        </CardTitle>
      </CardHeader>
      <CardContent className="grid grid-cols-2 gap-2">
        {actions.map((a) => (
          <Link key={a.href} to={a.href}>
            <div
              className="flex items-center gap-3 rounded-lg border border-border p-3 transition-all hover:border-primary/40 hover:bg-primary/5"
            >
              <a.icon className="h-4 w-4 text-primary" />
              <span className="text-sm font-medium">{a.label}</span>
            </div>
          </Link>
        ))}
      </CardContent>
    </Card>
  );
}

export function HomePage() {
  const { actor } = useAuth();
  const [stats, setStats] = useState<DashboardStats | null>(null);

  useEffect(() => {
    api.get<DashboardStats>("/dashboard/stats").then(setStats).catch(() => {});
  }, []);

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1>Оперативний облік</h1>
        {actor && (
          <p className="text-sm text-muted-foreground">
            {actor.org_label} — дашборд підготовки
          </p>
        )}
      </div>

      <div className="eyebrow">Загальні показники</div>

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <StatCard
          title="Підрозділи"
          value={stats?.total_orgs ?? null}
          icon={Building2}
          href={actor ? `/org/${actor.org_id}` : "#"}
          tint="#82a2b5"
        />
        <StatCard
          title="Групи підготовки"
          value={stats?.total_groups ?? null}
          icon={Users}
          href="/training"
          tint="#8fa565"
        />
        <StatCard
          title="Подання"
          value={stats?.total_submissions ?? null}
          icon={FileText}
          href="/training"
          tint="#c9a84c"
        />
        <StatCard
          title="Розбіжності"
          value={stats?.total_discrepancies ?? null}
          icon={AlertTriangle}
          href="/discrepancies"
          accent={(stats?.total_discrepancies ?? 0) > 0}
          tint="#c97a6e"
        />
      </div>

      <div className="grid gap-4 lg:grid-cols-3">
        <div className="flex flex-col gap-4 lg:col-span-2">
          {stats && <RecentActivity stats={stats} />}
          {!stats && <Skeleton className="h-64 w-full" />}
        </div>
        <div className="flex flex-col gap-4">
          {stats && <KindBreakdown data={stats.groups_by_kind} />}
          <QuickActions />
        </div>
      </div>
    </div>
  );
}
