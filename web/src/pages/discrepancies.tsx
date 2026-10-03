import { useEffect, useMemo, useState, useCallback } from "react";
import { Link, useNavigate } from "react-router-dom";
import { api } from "@/api/client";
import type { DiscrepancyRow, DiscrepancyComparison, ReportedGroupSnapshot } from "@/api/types";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import { Textarea } from "@/components/ui/textarea";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
} from "@/components/ui/sheet";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  AlertTriangle,
  Search,
  CheckCircle2,
  XCircle,
  ExternalLink,
  Clock,
  Building2,
  Hash,
  BarChart3,
  Calendar,
  Loader2,
  ArrowRight,
  GitCompareArrows,
  ChevronDown,
  ChevronUp,
} from "lucide-react";
import { Input } from "@/components/ui/input";

function kindLabel(kind: string): string {
  switch (kind) {
    case "horizontal": return "Горизонтальна";
    case "vertical": return "Вертикальна";
    case "temporal": return "Часова";
    case "data_quality": return "Якість даних";
    default: return kind;
  }
}

function statusLabel(status: string): string {
  switch (status) {
    case "open": return "Відкрита";
    case "resolved": return "Вирішена";
    case "notified": return "Повідомлено";
    case "in_progress": return "В роботі";
    case "dismissed": return "Відхилено";
    default: return status;
  }
}

function statusBadge(status: string) {
  switch (status) {
    case "open":
      return <Badge variant="destructive">Відкрита</Badge>;
    case "resolved":
      return <Badge variant="default">Вирішена</Badge>;
    case "notified":
      return <Badge style={{ background: "rgba(91,155,213,0.15)", color: "#5B9BD5", border: "1px solid rgba(91,155,213,0.3)" }}>Повідомлено</Badge>;
    case "in_progress":
      return <Badge style={{ background: "rgba(243,146,0,0.15)", color: "#F39200", border: "1px solid rgba(243,146,0,0.3)" }}>В роботі</Badge>;
    case "dismissed":
      return <Badge variant="secondary">Відхилено</Badge>;
    default:
      return <Badge variant="outline">{status}</Badge>;
  }
}

const KIND_STYLE: Record<string, { border: string; color: string; bg: string }> = {
  horizontal: { border: "rgba(91,155,213,0.4)", color: "#5B9BD5", bg: "rgba(91,155,213,0.1)" },
  temporal: { border: "rgba(201,168,76,0.4)", color: "#c9a84c", bg: "rgba(201,168,76,0.1)" },
  data_quality: { border: "rgba(143,165,101,0.4)", color: "#8fa565", bg: "rgba(143,165,101,0.1)" },
  vertical: { border: "rgba(201,122,110,0.4)", color: "#c97a6e", bg: "rgba(201,122,110,0.1)" },
};

const METRIC_LABELS: Record<string, string> = {
  arrived_count: "Прибуло",
  planned_count: "Заплановано",
  current_count: "Наявних",
  dropped_count: "Вибуло",
  completed_count: "Завершило",
  total_count: "Всього",
  in_training_count: "На навчанні",
};

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

function ValuesCompare({ values }: { values: DiscrepancyRow["values"] }) {
  if (values.length === 0) return <span className="text-muted-foreground">—</span>;

  return (
    <div className="flex flex-col gap-1.5">
      {values.map((v, i) => (
        <div
          key={i}
          className="flex items-center gap-2 rounded-md border px-3 py-2"
          style={{ borderColor: "var(--border)" }}
        >
          <span className="text-xs text-muted-foreground">{v.source_label}</span>
          <span className="ml-auto text-sm font-semibold tabular-nums">{v.value}</span>
        </div>
      ))}
    </div>
  );
}

const FIELD_LABELS: Record<string, string> = {
  training_kind: "Вид підготовки",
  vos_label: "ВОС",
  position_label: "Посада",
  course_label: "Курс",
  site_label: "Місце",
  organizer_label: "Організатор",
  planned_start: "Термін з",
  planned_end: "Термін по",
  equipment_text: "Озброєння/техніка",
  basis_doc_number: "Підстава",
  note: "Примітка",
  planned_count: "План",
  arrived_count: "Прибуло",
  in_training_count: "Навчаються",
};

const METRIC_TO_FIELD: Record<string, string> = {
  planned_count: "planned_count",
  arrived_count: "arrived_count",
  in_training_count: "in_training_count",
  planned_start: "planned_start",
  planned_end: "planned_end",
  site_id: "site_label",
};

type FieldKey = keyof ReportedGroupSnapshot;

const COMPARE_FIELDS: FieldKey[] = [
  "training_kind", "vos_label", "position_label", "course_label",
  "site_label", "organizer_label", "planned_start", "planned_end",
  "equipment_text", "basis_doc_number", "note",
  "planned_count", "arrived_count", "in_training_count",
];

function fieldValue(row: ReportedGroupSnapshot, field: FieldKey): string {
  const v = row[field];
  if (v == null || v === "") return "—";
  return String(v);
}

function ComparisonTable({ comparison }: { comparison: DiscrepancyComparison }) {
  const { rows, metric } = comparison;
  if (rows.length < 2) {
    return <p className="text-sm text-muted-foreground py-2">Недостатньо даних для порівняння.</p>;
  }

  const conflictField = METRIC_TO_FIELD[metric];

  return (
    <div className="overflow-x-auto -mx-4 px-4">
      <table className="w-full text-sm border-collapse">
        <thead>
          <tr>
            <th
              className="text-left py-1.5 px-2 text-[11px] font-semibold uppercase tracking-wider"
              style={{ color: "var(--muted-foreground)", fontFamily: "var(--font-heading)", minWidth: 100 }}
            >
              Поле
            </th>
            {rows.map((r) => (
              <th
                key={r.submission_id}
                className="text-left py-1.5 px-2 text-[11px] font-semibold uppercase tracking-wider"
                style={{ color: "var(--muted-foreground)", fontFamily: "var(--font-heading)", minWidth: 100 }}
              >
                {r.source_label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {COMPARE_FIELDS.map((field) => {
            const vals = rows.map(r => fieldValue(r, field));
            const isDiff = vals.some(v => v !== vals[0]);
            const isConflict = field === conflictField;

            return (
              <tr
                key={field}
                style={isConflict ? {
                  background: "rgba(217,83,79,0.08)",
                } : isDiff ? {
                  background: "rgba(243,146,0,0.06)",
                } : undefined}
              >
                <td
                  className="py-1.5 px-2 font-medium whitespace-nowrap"
                  style={{
                    borderBottom: "1px solid var(--border)",
                    color: isConflict ? "#D9534F" : undefined,
                    fontWeight: isConflict ? 700 : undefined,
                  }}
                >
                  {FIELD_LABELS[field] ?? field}
                  {isConflict && (
                    <span className="ml-1.5 text-[10px] uppercase tracking-wider" style={{ color: "#D9534F" }}>
                      конфлікт
                    </span>
                  )}
                </td>
                {rows.map((r) => (
                  <td
                    key={r.submission_id}
                    className="py-1.5 px-2 tabular-nums"
                    style={{
                      borderBottom: "1px solid var(--border)",
                      fontWeight: isConflict ? 700 : undefined,
                      color: isConflict ? "#D9534F" : isDiff ? "#F39200" : undefined,
                    }}
                  >
                    {fieldValue(r, field)}
                  </td>
                ))}
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

function DiscrepancyDetailSheet({
  disc,
  open,
  onOpenChange,
  onStatusChanged,
}: {
  disc: DiscrepancyRow | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onStatusChanged: () => void;
}) {
  const navigate = useNavigate();
  const [note, setNote] = useState("");
  const [updating, setUpdating] = useState(false);
  const [comparison, setComparison] = useState<DiscrepancyComparison | null>(null);
  const [compareLoading, setCompareLoading] = useState(false);
  const [compareOpen, setCompareOpen] = useState(false);

  const updateStatus = useCallback(async (newStatus: string) => {
    if (!disc) return;
    setUpdating(true);
    try {
      await api.patch(`/discrepancies/${disc.id}`, {
        status: newStatus,
        resolution_note: note.trim() || undefined,
      });
      onStatusChanged();
      onOpenChange(false);
    } catch {
      // error handled by api client
    } finally {
      setUpdating(false);
    }
  }, [disc, note, onStatusChanged, onOpenChange]);

  const loadComparison = useCallback(async () => {
    if (!disc) return;
    if (comparison && comparison.metric === disc.metric) {
      setCompareOpen(v => !v);
      return;
    }
    setCompareLoading(true);
    try {
      const data = await api.get<DiscrepancyComparison>(`/discrepancies/${disc.id}/compare`);
      setComparison(data);
      setCompareOpen(true);
    } catch {
      // error handled by api client
    } finally {
      setCompareLoading(false);
    }
  }, [disc, comparison]);

  useEffect(() => {
    if (!open) {
      setComparison(null);
      setCompareOpen(false);
    }
  }, [open]);

  if (!disc) return null;

  const ks = KIND_STYLE[disc.kind] ?? KIND_STYLE.vertical;
  const isActive = disc.status === "open" || disc.status === "in_progress" || disc.status === "notified";

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side="right" dockable className="w-full overflow-y-auto sm:max-w-lg">
        <SheetHeader>
          <SheetTitle className="flex items-center gap-2">
            <AlertTriangle className="h-5 w-5" style={{ color: ks.color }} />
            Розбіжність #{disc.id}
          </SheetTitle>
          <SheetDescription>{disc.org_label}</SheetDescription>
        </SheetHeader>

        <div className="flex flex-col gap-1 px-4">
          <DetailField icon={Building2} label="Підрозділ" value={
            <Link to={`/org/${disc.org_id}`} className="text-primary hover:underline">
              {disc.org_label}
            </Link>
          } />
          <DetailField icon={Hash} label="Тип" value={
            <Badge variant="outline" style={{ borderColor: ks.border, color: ks.color, background: ks.bg }}>
              {kindLabel(disc.kind)}
            </Badge>
          } />
          <DetailField icon={BarChart3} label="Метрика" value={METRIC_LABELS[disc.metric] ?? disc.metric_label} />
          {disc.group_label && (
            <DetailField icon={Hash} label="Група" value={disc.group_label} />
          )}
          <DetailField icon={Calendar} label="Станом на" value={disc.as_of} />
          <DetailField icon={Clock} label="Створено" value={disc.created_at} />
          <DetailField icon={AlertTriangle} label="Статус" value={statusBadge(disc.status)} />
        </div>

        <Separator className="mx-4" />

        <div className="flex flex-col gap-3 px-4">
          <span
            className="text-[11px] font-semibold uppercase tracking-wider"
            style={{ color: "var(--muted-foreground)", fontFamily: "var(--font-heading)" }}
          >
            Порівняння значень
          </span>
          <ValuesCompare values={disc.values} />
        </div>

        {disc.values.length >= 2 && (
          <>
            <Separator className="mx-4" />
            <div className="flex flex-col gap-3 px-4">
              <Button
                variant="outline"
                className="w-full justify-between"
                onClick={loadComparison}
                disabled={compareLoading}
              >
                <span className="flex items-center gap-2">
                  {compareLoading ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    <GitCompareArrows className="h-4 w-4" />
                  )}
                  Порівняти подання
                </span>
                {compareOpen ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
              </Button>
              {compareOpen && comparison && (
                <ComparisonTable comparison={comparison} />
              )}
            </div>
          </>
        )}

        {disc.group_id && (
          <>
            <Separator className="mx-4" />
            <div className="px-4">
              <Button
                variant="outline"
                className="w-full justify-between"
                onClick={() => {
                  onOpenChange(false);
                  navigate(`/data?group=${disc.group_id}`);
                }}
              >
                <span className="flex items-center gap-2">
                  <ExternalLink className="h-4 w-4" />
                  Перейти до даних групи
                </span>
                <ArrowRight className="h-4 w-4" />
              </Button>
            </div>
          </>
        )}

        {isActive && (
          <>
            <Separator className="mx-4" />
            <div className="flex flex-col gap-3 px-4 pb-4">
              <span
                className="text-[11px] font-semibold uppercase tracking-wider"
                style={{ color: "var(--muted-foreground)", fontFamily: "var(--font-heading)" }}
              >
                Дії
              </span>
              <Textarea
                placeholder="Коментар (необов'язково)"
                value={note}
                onChange={(e) => setNote(e.target.value)}
                rows={2}
              />
              <div className="flex gap-2">
                {disc.status !== "in_progress" && (
                  <Button
                    variant="outline"
                    size="sm"
                    className="flex-1"
                    onClick={() => updateStatus("in_progress")}
                    disabled={updating}
                  >
                    {updating ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <Clock className="mr-2 h-4 w-4" />}
                    В роботу
                  </Button>
                )}
                <Button
                  variant="default"
                  size="sm"
                  className="flex-1"
                  onClick={() => updateStatus("resolved")}
                  disabled={updating}
                >
                  {updating ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <CheckCircle2 className="mr-2 h-4 w-4" />}
                  Вирішено
                </Button>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => updateStatus("dismissed")}
                  disabled={updating}
                >
                  {updating ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <XCircle className="mr-2 h-4 w-4" />}
                  Відхилити
                </Button>
              </div>
            </div>
          </>
        )}
      </SheetContent>
    </Sheet>
  );
}

export function DiscrepanciesPage() {
  const [rows, setRows] = useState<DiscrepancyRow[] | null>(null);
  const [statusFilter, setStatusFilter] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [selectedDisc, setSelectedDisc] = useState<DiscrepancyRow | null>(null);
  const [sheetOpen, setSheetOpen] = useState(false);

  const loadData = useCallback(() => {
    api.get<DiscrepancyRow[]>("/discrepancies").then(setRows).catch(() => setRows([]));
  }, []);

  useEffect(() => { loadData(); }, [loadData]);

  const statusCounts = useMemo(() => {
    if (!rows) return new Map<string, number>();
    const counts = new Map<string, number>();
    for (const r of rows) {
      counts.set(r.status, (counts.get(r.status) ?? 0) + 1);
    }
    return counts;
  }, [rows]);

  const filtered = useMemo(() => {
    if (!rows) return [];
    let result = rows;
    if (statusFilter) result = result.filter((r) => r.status === statusFilter);
    if (search) {
      const q = search.toLowerCase();
      result = result.filter((r) =>
        r.org_label.toLowerCase().includes(q) ||
        r.metric_label.toLowerCase().includes(q) ||
        (r.group_label ?? "").toLowerCase().includes(q)
      );
    }
    return result;
  }, [rows, statusFilter, search]);

  const openCount = statusCounts.get("open") ?? 0;

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-bold">Розбіжності</h1>
        {rows !== null && rows.length > 0 && (
          <div className="flex items-baseline gap-2">
            <span className="text-2xl font-bold tabular-nums" style={{ color: openCount > 0 ? "#D9534F" : "var(--foreground)" }}>
              {openCount}
            </span>
            <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              відкритих
            </span>
          </div>
        )}
      </div>

      {/* Filters */}
      {rows !== null && rows.length > 0 && (
        <div className="flex flex-wrap items-center gap-2">
          <div className="relative w-full max-w-xs">
            <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              placeholder="Пошук за підрозділом, метрикою..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-9"
            />
          </div>
          <button
            onClick={() => setStatusFilter(null)}
            className="rounded-md border px-2.5 py-1 text-xs font-semibold uppercase tracking-wider transition-colors"
            style={{
              fontFamily: "var(--font-heading)",
              borderColor: !statusFilter ? "var(--primary)" : "var(--border)",
              color: !statusFilter ? "var(--primary)" : "var(--muted-foreground)",
              background: !statusFilter ? "color-mix(in srgb, var(--primary) 8%, transparent)" : "transparent",
              cursor: "pointer",
            }}
          >
            Усі ({rows.length})
          </button>
          {Array.from(statusCounts.entries())
            .sort((a, b) => b[1] - a[1])
            .map(([status, count]) => (
              <button
                key={status}
                onClick={() => setStatusFilter(statusFilter === status ? null : status)}
                className="rounded-md border px-2.5 py-1 text-xs font-semibold transition-colors"
                style={{
                  fontFamily: "var(--font-heading)",
                  letterSpacing: "0.02em",
                  borderColor: statusFilter === status ? "var(--primary)" : "var(--border)",
                  color: statusFilter === status ? "var(--primary)" : "var(--muted-foreground)",
                  background: statusFilter === status ? "color-mix(in srgb, var(--primary) 8%, transparent)" : "transparent",
                  cursor: "pointer",
                }}
              >
                {statusLabel(status)} ({count})
              </button>
            ))}
        </div>
      )}

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <AlertTriangle className="h-4 w-4" />
            Виявлені розбіжності між рівнями
            {filtered.length !== (rows?.length ?? 0) && (
              <span className="text-sm font-normal text-muted-foreground">
                ({filtered.length} з {rows?.length})
              </span>
            )}
          </CardTitle>
        </CardHeader>
        <CardContent>
          {rows === null ? (
            <div className="flex flex-col gap-2">
              <Skeleton className="h-8 w-full" />
              <Skeleton className="h-8 w-full" />
              <Skeleton className="h-8 w-full" />
            </div>
          ) : filtered.length === 0 ? (
            <p className="py-8 text-center text-sm text-muted-foreground">
              {rows.length === 0 ? "Розбіжностей не знайдено" : "Нічого не знайдено за цим фільтром"}
            </p>
          ) : (
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Підрозділ</TableHead>
                    <TableHead>Тип</TableHead>
                    <TableHead>Метрика</TableHead>
                    <TableHead>Група</TableHead>
                    <TableHead>Значення</TableHead>
                    <TableHead>Статус</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {filtered.map((r) => {
                    const ks = KIND_STYLE[r.kind] ?? KIND_STYLE.vertical;
                    return (
                      <TableRow
                        key={r.id}
                        className="cursor-pointer"
                        onClick={() => { setSelectedDisc(r); setSheetOpen(true); }}
                      >
                        <TableCell className="font-medium">
                          {r.org_label}
                        </TableCell>
                        <TableCell>
                          <Badge
                            variant="outline"
                            style={{ borderColor: ks.border, color: ks.color, background: ks.bg }}
                          >
                            {kindLabel(r.kind)}
                          </Badge>
                        </TableCell>
                        <TableCell>{METRIC_LABELS[r.metric] ?? r.metric_label}</TableCell>
                        <TableCell className="text-sm text-muted-foreground">
                          {r.group_label ?? "—"}
                        </TableCell>
                        <TableCell>
                          <div className="flex flex-col gap-0.5 text-sm">
                            {r.values.map((v, i) => (
                              <span key={i}>
                                <span className="text-muted-foreground">{v.source_label}:</span> {v.value}
                              </span>
                            ))}
                          </div>
                        </TableCell>
                        <TableCell>{statusBadge(r.status)}</TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            </div>
          )}
        </CardContent>
      </Card>

      <DiscrepancyDetailSheet
        disc={selectedDisc}
        open={sheetOpen}
        onOpenChange={(v) => { setSheetOpen(v); if (!v) setSelectedDisc(null); }}
        onStatusChanged={loadData}
      />
    </div>
  );
}
