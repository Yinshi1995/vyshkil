import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { KindBadge } from "@/components/kind-badge";
import { api } from "@/api/client";
import type { AdminGroupRow, AdminSubmissionRow } from "@/api/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableHeader, TableBody, TableRow, TableHead, TableCell } from "@/components/ui/table";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
} from "@/components/ui/sheet";
import {
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
  Search,
  Upload,
  FileText,
  Users,
  Clock,
  MapPin,
  Calendar,
  Hash,
  Copy,
  Eye,
} from "lucide-react";
import { useContextMenu, ContextMenuPortal, type ContextMenuEntry } from "@/components/context-menu";
import { sourceTypeLabel, statusLabel, statusVariant } from "@/lib/labels";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type SortKey = keyof AdminGroupRow;
type SortDir = "asc" | "desc";

// ---------------------------------------------------------------------------
// Column definitions
// ---------------------------------------------------------------------------

interface ColDef {
  key: SortKey;
  label: string;
  align?: "right";
  width?: string;
}

const COLUMNS: ColDef[] = [
  { key: "org_label", label: "Підрозділ", width: "minmax(160px, 1.5fr)" },
  { key: "training_kind", label: "Вид", width: "100px" },
  { key: "vos_label", label: "ВОС", width: "minmax(100px, 1fr)" },
  { key: "site_label", label: "Полігон", width: "minmax(100px, 1fr)" },
  { key: "planned_start", label: "Початок", width: "95px" },
  { key: "planned_end", label: "Кінець", width: "95px" },
  { key: "planned_count", label: "План", align: "right", width: "70px" },
  { key: "arrived_count", label: "Прибуло", align: "right", width: "75px" },
  { key: "in_training_count", label: "Навч.", align: "right", width: "70px" },
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function compare(a: AdminGroupRow, b: AdminGroupRow, key: SortKey, dir: SortDir): number {
  const va = a[key];
  const vb = b[key];
  let cmp: number;
  if (typeof va === "number" && typeof vb === "number") {
    cmp = va - vb;
  } else {
    cmp = String(va ?? "").localeCompare(String(vb ?? ""), "uk");
  }
  return dir === "asc" ? cmp : -cmp;
}

function matchesSearch(g: AdminGroupRow, q: string): boolean {
  const lower = q.toLowerCase();
  return (
    g.org_label.toLowerCase().includes(lower) ||
    g.training_kind.toLowerCase().includes(lower) ||
    g.vos_label.toLowerCase().includes(lower) ||
    g.site_label.toLowerCase().includes(lower)
  );
}

// ---------------------------------------------------------------------------
// Stat pill
// ---------------------------------------------------------------------------

function Stat({ label, value }: { label: string; value: number }) {
  return (
    <div className="flex items-baseline gap-1.5">
      <span className="text-2xl font-bold tabular-nums" style={{ color: "var(--foreground)" }}>
        {value.toLocaleString("uk-UA")}
      </span>
      <span
        className="text-xs font-semibold uppercase tracking-wider"
        style={{ color: "var(--muted-foreground)", fontFamily: "var(--font-heading)" }}
      >
        {label}
      </span>
    </div>
  );
}


// ---------------------------------------------------------------------------
// Sort icon
// ---------------------------------------------------------------------------

function SortIcon({ active, dir }: { active: boolean; dir: SortDir }) {
  if (!active) return <ArrowUpDown className="ml-1 inline h-3 w-3 opacity-30" />;
  if (dir === "asc") return <ArrowUp className="ml-1 inline h-3 w-3 text-primary" />;
  return <ArrowDown className="ml-1 inline h-3 w-3 text-primary" />;
}

// ---------------------------------------------------------------------------
// Detail Sheet
// ---------------------------------------------------------------------------

function DetailField({
  icon: Icon,
  label,
  value,
}: {
  icon: React.ElementType;
  label: string;
  value: React.ReactNode;
}) {
  return (
    <div className="flex items-start gap-3 py-2">
      <Icon className="mt-0.5 h-4 w-4 shrink-0" style={{ color: "var(--muted-foreground)" }} />
      <div className="flex flex-col gap-0.5">
        <span
          className="text-[11px] font-semibold uppercase tracking-wider"
          style={{ color: "var(--muted-foreground)", fontFamily: "var(--font-heading)" }}
        >
          {label}
        </span>
        <span className="text-sm">{value}</span>
      </div>
    </div>
  );
}

function CountBox({ label, value, accent }: { label: string; value: number; accent?: boolean }) {
  return (
    <div
      className="flex flex-col items-center rounded-lg border px-4 py-3"
      style={{
        borderColor: accent ? "var(--primary)" : "var(--border)",
        background: accent ? "color-mix(in srgb, var(--primary) 5%, transparent)" : "transparent",
      }}
    >
      <span
        className="text-xl font-bold tabular-nums"
        style={{ color: accent ? "var(--primary)" : "var(--foreground)" }}
      >
        {value}
      </span>
      <span
        className="text-[11px] font-semibold uppercase tracking-wider"
        style={{ color: "var(--muted-foreground)", fontFamily: "var(--font-heading)" }}
      >
        {label}
      </span>
    </div>
  );
}

function GroupDetailSheet({
  group,
  open,
  onOpenChange,
}: {
  group: AdminGroupRow | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  if (!group) return null;

  const pct = group.planned_count > 0
    ? Math.round((group.in_training_count / group.planned_count) * 100)
    : 0;

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side="right" dockable className="w-full overflow-y-auto sm:max-w-lg">
        <SheetHeader>
          <SheetTitle>{group.org_label}</SheetTitle>
          <SheetDescription>
            Група підготовки #{group.id}
          </SheetDescription>
        </SheetHeader>

        <div className="flex flex-col gap-1 px-4">
          <DetailField icon={FileText} label="Вид підготовки" value={<KindBadge kind={group.training_kind} />} />
          <DetailField icon={Hash} label="ВОС / Спеціальність" value={group.vos_label || "—"} />
          <DetailField icon={MapPin} label="Полігон / Місце" value={group.site_label || "—"} />
          <DetailField
            icon={Calendar}
            label="Період"
            value={
              group.planned_start && group.planned_end
                ? `${group.planned_start} — ${group.planned_end}`
                : group.planned_start || "—"
            }
          />
        </div>

        <Separator className="mx-4" />

        <div className="flex flex-col gap-3 px-4 pb-4">
          <span
            className="text-[11px] font-semibold uppercase tracking-wider"
            style={{ color: "var(--muted-foreground)", fontFamily: "var(--font-heading)" }}
          >
            Кількості
          </span>
          <div className="grid grid-cols-3 gap-2">
            <CountBox label="План" value={group.planned_count} accent />
            <CountBox label="Прибуло" value={group.arrived_count} />
            <CountBox label="Навчається" value={group.in_training_count} />
          </div>
          {group.planned_count > 0 && (
            <div className="flex items-center gap-2">
              <div className="flex-1 overflow-hidden rounded-full" style={{ height: 6, background: "var(--muted)" }}>
                <div
                  className="h-full rounded-full transition-all duration-500"
                  style={{
                    width: `${Math.min(100, pct)}%`,
                    background: "var(--primary)",
                  }}
                />
              </div>
              <span className="text-xs tabular-nums" style={{ color: "var(--muted-foreground)" }}>
                {pct}%
              </span>
            </div>
          )}
        </div>
      </SheetContent>
    </Sheet>
  );
}

// ---------------------------------------------------------------------------
// Import link (actual import lives on data workspace)
// ---------------------------------------------------------------------------

function ImportLink() {
  return (
    <Card className="card-animate card-hover">
      <CardContent className="flex items-center gap-3 py-4">
        <Upload className="h-5 w-5 text-primary" />
        <div className="flex flex-col gap-0.5">
          <span className="text-sm font-medium">Імпорт з файлу</span>
          <span className="text-xs text-muted-foreground">
            Завантаження xlsx-файлів (Фах, БпС, Терміни, КВід, ІВС) —{" "}
            <Link to="/data" className="text-primary hover:underline">
              Робоча область даних
            </Link>
          </span>
        </div>
      </CardContent>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// Recent submissions
// ---------------------------------------------------------------------------

function RecentSubmissions() {
  const [subs, setSubs] = useState<AdminSubmissionRow[] | null>(null);

  useEffect(() => {
    api.get<AdminSubmissionRow[]>("/submissions/recent").then(setSubs).catch(() => setSubs([]));
  }, []);

  if (subs === null) return <Skeleton className="h-32 w-full" />;
  if (subs.length === 0) return null;

  return (
    <Card className="card-animate">
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <Clock className="h-4 w-4" />
          Останні подання ({subs.length})
        </CardTitle>
      </CardHeader>
      <CardContent className="overflow-x-auto">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Підрозділ</TableHead>
              <TableHead>Тип</TableHead>
              <TableHead>Статус</TableHead>
              <TableHead className="hidden sm:table-cell">Станом на</TableHead>
              <TableHead className="hidden sm:table-cell">Оновлено</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {subs.map((s) => (
              <TableRow key={s.id}>
                <TableCell className="font-medium">{s.org_label}</TableCell>
                <TableCell>{sourceTypeLabel(s.source_type)}</TableCell>
                <TableCell>
                  <Badge variant={statusVariant(s.status)}>
                    {statusLabel(s.status)}
                  </Badge>
                </TableCell>
                <TableCell className="hidden sm:table-cell">{s.as_of_date}</TableCell>
                <TableCell className="hidden sm:table-cell">{s.updated_at}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// Main page
// ---------------------------------------------------------------------------

export function TrainingPage() {
  const [groups, setGroups] = useState<AdminGroupRow[] | null>(null);
  const [sortKey, setSortKey] = useState<SortKey>("org_label");
  const [sortDir, setSortDir] = useState<SortDir>("asc");
  const [search, setSearch] = useState("");
  const [kindFilter, setKindFilter] = useState<string | null>(null);
  const [selectedGroup, setSelectedGroup] = useState<AdminGroupRow | null>(null);
  const [sheetOpen, setSheetOpen] = useState(false);
  const ctxMenu = useContextMenu();

  useEffect(() => {
    api.get<AdminGroupRow[]>("/training/groups").then(setGroups).catch(() => setGroups([]));
  }, []);

  function toggleSort(key: SortKey) {
    if (sortKey === key) {
      setSortDir((d) => (d === "asc" ? "desc" : "asc"));
    } else {
      setSortKey(key);
      setSortDir("asc");
    }
  }

  const filtered = useMemo(() => {
    if (!groups) return [];
    let rows = groups;
    if (kindFilter) rows = rows.filter((g) => g.training_kind === kindFilter);
    if (search) rows = rows.filter((g) => matchesSearch(g, search));
    return [...rows].sort((a, b) => compare(a, b, sortKey, sortDir));
  }, [groups, kindFilter, search, sortKey, sortDir]);

  const kinds = useMemo(() => {
    if (!groups) return [];
    const counts = new Map<string, number>();
    for (const g of groups) {
      const k = g.training_kind || "—";
      counts.set(k, (counts.get(k) ?? 0) + 1);
    }
    return Array.from(counts.entries()).sort((a, b) => b[1] - a[1]);
  }, [groups]);

  function handleRowContextMenu(e: React.MouseEvent, g: AdminGroupRow) {
    const items: ContextMenuEntry[] = [
      {
        label: "Деталі",
        icon: <Eye className="h-4 w-4" />,
        onClick: () => { setSelectedGroup(g); setSheetOpen(true); },
      },
      { separator: true },
      {
        label: "Копіювати підрозділ",
        icon: <Copy className="h-4 w-4" />,
        onClick: () => { navigator.clipboard.writeText(g.org_label); },
      },
    ];
    ctxMenu.open(e, items);
  }

  const totalPlanned = filtered.reduce((s, g) => s + g.planned_count, 0);
  const totalArrived = filtered.reduce((s, g) => s + g.arrived_count, 0);
  const totalTraining = filtered.reduce((s, g) => s + g.in_training_count, 0);

  return (
    <div className="flex flex-col gap-5">
      {/* Header + stats */}
      <div className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <h1 className="text-2xl font-bold">Підготовка</h1>
          {groups !== null && groups.length > 0 && (
            <div className="flex flex-wrap gap-5">
              <Stat label="груп" value={filtered.length} />
              <Stat label="план" value={totalPlanned} />
              <Stat label="прибуло" value={totalArrived} />
              <Stat label="навч." value={totalTraining} />
            </div>
          )}
        </div>

        {/* Search + kind filters */}
        <div className="flex flex-wrap items-center gap-2">
          <div className="relative w-full max-w-xs">
            <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              placeholder="Пошук за підрозділом, ВОС, полігоном..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-9"
            />
          </div>
          {kinds.length > 1 && (
            <>
              <Button
                variant={!kindFilter ? "default" : "outline"}
                size="sm"
                onClick={() => setKindFilter(null)}
                className="text-xs font-semibold uppercase tracking-wider"
                style={{ fontFamily: "var(--font-heading)" }}
              >
                Усі
              </Button>
              {kinds.map(([kind, count]) => (
                <Button
                  key={kind}
                  variant={kindFilter === kind ? "default" : "outline"}
                  size="sm"
                  onClick={() => setKindFilter(kindFilter === kind ? null : kind)}
                  className="text-xs font-semibold"
                  style={{ fontFamily: "var(--font-heading)", letterSpacing: "0.02em" }}
                >
                  {kind} ({count})
                </Button>
              ))}
            </>
          )}
        </div>
      </div>

      {/* Data grid */}
      {groups === null ? (
        <div className="flex flex-col gap-2">
          {Array.from({ length: 8 }).map((_, i) => (
            <Skeleton key={i} className="h-10 w-full" />
          ))}
        </div>
      ) : filtered.length === 0 ? (
        <div className="flex flex-col items-center gap-3 py-16">
          <Users className="h-10 w-10 text-muted-foreground" />
          <p className="text-sm text-muted-foreground">
            {search || kindFilter ? "Нічого не знайдено за цим фільтром" : "Груп підготовки ще немає"}
          </p>
        </div>
      ) : (
        <div className="overflow-x-auto">
        <Table>
          <TableHeader>
            <TableRow>
              {COLUMNS.map((col) => (
                <TableHead
                  key={col.key}
                  onClick={() => toggleSort(col.key)}
                  className="cursor-pointer select-none whitespace-nowrap"
                  style={{ textAlign: col.align ?? "left" }}
                >
                  {col.label}
                  <SortIcon active={sortKey === col.key} dir={sortDir} />
                </TableHead>
              ))}
            </TableRow>
          </TableHeader>
          <TableBody>
            {filtered.map((g) => (
              <TableRow
                key={g.id}
                onClick={() => { setSelectedGroup(g); setSheetOpen(true); }}
                onContextMenu={(e) => handleRowContextMenu(e, g)}
                className="cursor-pointer"
              >
                <TableCell className="font-medium">{g.org_label}</TableCell>
                <TableCell><KindBadge kind={g.training_kind} /></TableCell>
                <TableCell className="text-sm">{g.vos_label}</TableCell>
                <TableCell className="text-sm">{g.site_label}</TableCell>
                <TableCell>{g.planned_start}</TableCell>
                <TableCell>{g.planned_end}</TableCell>
                <TableCell style={{ textAlign: "right" }}>
                  <span className="tabular-nums font-semibold">{g.planned_count}</span>
                </TableCell>
                <TableCell style={{ textAlign: "right" }}>
                  <span className="tabular-nums">{g.arrived_count}</span>
                </TableCell>
                <TableCell style={{ textAlign: "right" }}>
                  <span className="tabular-nums">{g.in_training_count}</span>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
        </div>
      )}

      {/* Detail side panel */}
      <GroupDetailSheet
        group={selectedGroup}
        open={sheetOpen}
        onOpenChange={(v) => { setSheetOpen(v); if (!v) setSelectedGroup(null); }}
      />

      {/* Import */}
      <ImportLink />

      {/* Recent submissions */}
      <RecentSubmissions />

      <ContextMenuPortal state={ctxMenu.state} onClose={ctxMenu.close} />
    </div>
  );
}
