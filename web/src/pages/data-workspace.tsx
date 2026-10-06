import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useSearchParams, Link } from "react-router-dom";
import { toast } from "sonner";
import { useAuth } from "@/context/auth";
import { todayIso } from "@/lib/date-ua";
import { KindBadge } from "@/components/kind-badge";
import {
  createColumnHelper,
  flexRender,
  getCoreRowModel,
  getExpandedRowModel,
  getFilteredRowModel,
  getGroupedRowModel,
  getSortedRowModel,
  useReactTable,
  type ColumnDef,
  type SortingState,
  type GroupingState,
  type ColumnFiltersState,
} from "@tanstack/react-table";
import { useVirtualizer } from "@tanstack/react-virtual";
import { api } from "@/api/client";
import type {
  DataGroupRow,
  GroupEventRow,
  VenueSearchRow,
  CityRow,
  AdminSubmissionRow,
} from "@/api/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { DateInputUa } from "@/components/ui/date-input-ua";
import { Calendar } from "@/components/ui/calendar";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { format, parse } from "date-fns";
import { uk } from "date-fns/locale";
import { Badge } from "@/components/ui/badge";
import { Textarea } from "@/components/ui/textarea";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Skeleton } from "@/components/ui/skeleton";
import { sourceTypeLabel, statusLabel, statusVariant } from "@/lib/labels";
import {
  Command,
  CommandInput,
  CommandList,
  CommandEmpty,
  CommandGroup,
  CommandItem,
} from "@/components/ui/command";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
} from "@/components/ui/sheet";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
  Search,
  Plus,
  Trash2,
  MoreHorizontal,
  ChevronDown,
  ChevronRight,
  Calendar as CalendarIcon,
  Hash,
  MapPin,
  FileText,
  Loader2,
  X,
  Users,
  Layers,
  Upload,
  FileUp,
  AlertTriangle,
  Building2,
  Copy,
  Eye,
  Clock,
  Download,
  CheckCircle2,
  FileWarning,
  GraduationCap,
} from "lucide-react";
import { useContextMenu, ContextMenuPortal, type ContextMenuEntry } from "@/components/context-menu";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------


const EVENT_LABELS: Record<string, { label: string; icon: string; color: string }> = {
  planned: { label: "План", icon: "📋", color: "var(--muted-foreground)" },
  arrived: { label: "Прибуло", icon: "🚌", color: "#5B9BD5" },
  started: { label: "Розпочали", icon: "▶", color: "#8FB339" },
  added: { label: "Додано", icon: "+", color: "#8FB339" },
  attrition: { label: "Вибуло", icon: "−", color: "#D9534F" },
  completed: { label: "Завершили", icon: "✓", color: "#F39200" },
  vos_awarded: { label: "ВОС присвоєно", icon: "★", color: "#F39200" },
  vos_not_awarded: { label: "ВОС не присвоєно", icon: "✗", color: "#D9534F" },
  correction: { label: "Корекція", icon: "~", color: "var(--muted-foreground)" },
};

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------



function validateCount(s: string, required: boolean): string | null {
  if (!s && required) return "Обов'язкове поле";
  if (!s) return null;
  const n = parseInt(s, 10);
  if (isNaN(n) || n < 0) return "Число ≥ 0";
  return null;
}

function FieldError({ error }: { error?: string }) {
  if (!error) return null;
  return (
    <span className="mt-0.5 block text-xs" style={{ color: "#D9534F" }}>
      {error}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function Stat({ label, value }: { label: string; value: number | string }) {
  return (
    <div className="flex items-baseline gap-1.5">
      <span
        className="text-2xl font-bold tabular-nums"
        style={{ color: "var(--foreground)" }}
      >
        {typeof value === "number" ? value.toLocaleString("uk-UA") : value}
      </span>
      <span
        className="text-[11px] font-semibold uppercase tracking-wider"
        style={{ color: "var(--muted-foreground)" }}
      >
        {label}
      </span>
    </div>
  );
}

function SortIcon({ sorted }: { sorted: false | "asc" | "desc" }) {
  if (!sorted)
    return <ArrowUpDown className="ml-1 inline h-3 w-3 opacity-30" />;
  if (sorted === "asc")
    return <ArrowUp className="ml-1 inline h-3 w-3 text-primary" />;
  return <ArrowDown className="ml-1 inline h-3 w-3 text-primary" />;
}

// ---------------------------------------------------------------------------
// Editable cell
// ---------------------------------------------------------------------------

function EditableCell({
  value,
  onSave,
  type = "text",
}: {
  value: string | number;
  onSave: (v: string) => void;
  type?: "text" | "number";
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(String(value));
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (editing) {
      inputRef.current?.focus();
      inputRef.current?.select();
    }
  }, [editing]);

  if (!editing) {
    return (
      <span
        className="block w-full cursor-text truncate rounded px-1 py-0.5 hover:bg-accent/50"
        onDoubleClick={() => {
          setDraft(String(value));
          setEditing(true);
        }}
      >
        {value || <span className="text-muted-foreground">—</span>}
      </span>
    );
  }

  return (
    <input
      ref={inputRef}
      className="w-full rounded border border-primary bg-background px-1 py-0.5 text-sm outline-none"
      type={type}
      value={draft}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={() => {
        setEditing(false);
        if (draft !== String(value)) onSave(draft);
      }}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          setEditing(false);
          if (draft !== String(value)) onSave(draft);
        }
        if (e.key === "Escape") {
          setEditing(false);
          setDraft(String(value));
        }
      }}
    />
  );
}

// ---------------------------------------------------------------------------
// Column definitions
// ---------------------------------------------------------------------------

const col = createColumnHelper<DataGroupRow>();

function buildColumns(
  onUpdate: (id: number, field: string, value: string) => void,
  onDelete: (row: DataGroupRow) => void,
  onDetail: (row: DataGroupRow) => void,
  isAdmin: boolean,
): ColumnDef<DataGroupRow, unknown>[] {
  return [
    col.accessor("org_label", {
      header: "Підрозділ",
      size: 140,
      cell: (info) => {
        const disc = info.row.original.discrepancy_count;
        return (
          <div className="flex items-center gap-1">
            {isAdmin ? (
              <EditableCell
                value={info.getValue()}
                onSave={(v) =>
                  onUpdate(info.row.original.id, "sender_org_id", v)
                }
              />
            ) : (
              <span className="block w-full truncate px-1 py-0.5">{info.getValue()}</span>
            )}
            {disc > 0 && (
              <span
                title={`${disc} розбіжн.`}
                className="shrink-0"
                style={{ color: "#D9534F" }}
              >
                <AlertTriangle className="h-3.5 w-3.5" />
              </span>
            )}
          </div>
        );
      },
    }) as ColumnDef<DataGroupRow, unknown>,
    col.accessor("training_kind", {
      header: "Вид",
      size: 100,
      cell: (info) => <KindBadge kind={info.getValue()} />,
      filterFn: "equals",
    }) as ColumnDef<DataGroupRow, unknown>,
    col.accessor("vos_label", {
      header: "ВОС / курс",
      size: 180,
      cell: (info) => (
        <span className="truncate text-sm">{info.getValue() || "—"}</span>
      ),
    }) as ColumnDef<DataGroupRow, unknown>,
    col.accessor("site_label", {
      header: "Місце",
      size: 180,
      cell: (info) => {
        const row = info.row.original;
        const parts = [info.getValue(), row.city_label].filter(Boolean);
        return <span className="truncate text-sm">{parts.join(", ") || "—"}</span>;
      },
    }) as ColumnDef<DataGroupRow, unknown>,
    col.accessor("planned_start", {
      header: "З",
      size: 90,
      cell: (info) => (
        <EditableCell
          value={info.getValue()}
          onSave={(v) =>
            onUpdate(info.row.original.id, "planned_start", v)
          }
        />
      ),
    }) as ColumnDef<DataGroupRow, unknown>,
    col.accessor("planned_end", {
      header: "По",
      size: 90,
      cell: (info) => (
        <EditableCell
          value={info.getValue()}
          onSave={(v) =>
            onUpdate(info.row.original.id, "planned_end", v)
          }
        />
      ),
    }) as ColumnDef<DataGroupRow, unknown>,
    col.accessor("planned_count", {
      header: "План",
      size: 60,
      meta: { align: "right" },
      cell: (info) => (
        <span className="tabular-nums font-semibold">{info.getValue()}</span>
      ),
    }) as ColumnDef<DataGroupRow, unknown>,
    col.accessor("arrived_count", {
      header: "Приб.",
      size: 55,
      meta: { align: "right" },
      cell: (info) => (
        <span className="tabular-nums">{info.getValue()}</span>
      ),
    }) as ColumnDef<DataGroupRow, unknown>,
    col.accessor("in_training_count", {
      header: "Навч.",
      size: 55,
      meta: { align: "right" },
      cell: (info) => (
        <span className="tabular-nums">{info.getValue()}</span>
      ),
    }) as ColumnDef<DataGroupRow, unknown>,
    col.accessor("completed_count", {
      header: "Зав.",
      size: 50,
      meta: { align: "right" },
      cell: (info) => (
        <span className="tabular-nums text-muted-foreground">
          {info.getValue()}
        </span>
      ),
    }) as ColumnDef<DataGroupRow, unknown>,
    col.accessor("attrition_count", {
      header: "Виб.",
      size: 50,
      meta: { align: "right" },
      cell: (info) => {
        const v = info.getValue();
        return (
          <span
            className="tabular-nums"
            style={{ color: v > 0 ? "#D9534F" : "var(--muted-foreground)" }}
          >
            {v}
          </span>
        );
      },
    }) as ColumnDef<DataGroupRow, unknown>,
    col.display({
      id: "actions",
      size: 44,
      cell: (info) => (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="icon" className="h-7 w-7">
              <MoreHorizontal className="h-4 w-4 text-muted-foreground" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onClick={() => onDetail(info.row.original)}>
              <FileText className="mr-2 h-4 w-4" />
              Деталі
            </DropdownMenuItem>
            {isAdmin && (
              <>
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  className="text-destructive"
                  onClick={() => onDelete(info.row.original)}
                >
                  <Trash2 className="mr-2 h-4 w-4" />
                  Видалити
                </DropdownMenuItem>
              </>
            )}
          </DropdownMenuContent>
        </DropdownMenu>
      ),
    }) as ColumnDef<DataGroupRow, unknown>,
  ];
}

// ---------------------------------------------------------------------------
// Detail Panel
// ---------------------------------------------------------------------------

interface AttritionReasonOption {
  id: number;
  name: string;
}

const EVENT_TYPE_OPTIONS = [
  { value: "planned", label: "План" },
  { value: "arrived", label: "Прибуло" },
  { value: "started", label: "Розпочали" },
  { value: "added", label: "Додано" },
  { value: "attrition", label: "Вибуло" },
  { value: "completed", label: "Завершили" },
  { value: "vos_awarded", label: "ВОС присвоєно" },
  { value: "correction", label: "Корекція" },
];


function EventTimeline({
  groupId,
  open,
  onChanged,
}: {
  groupId: number | null;
  open: boolean;
  onChanged?: () => void;
}) {
  const [events, setEvents] = useState<GroupEventRow[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [tick, setTick] = useState(0);

  // Add-event form state
  const [showForm, setShowForm] = useState(false);
  const [evType, setEvType] = useState("arrived");
  const [evCount, setEvCount] = useState("");
  const [evDate, setEvDate] = useState(todayIso);
  const [evReasonId, setEvReasonId] = useState<number | null>(null);
  const [evNote, setEvNote] = useState("");
  const [saving, setSaving] = useState(false);

  // Attrition reasons (loaded on demand)
  const [reasons, setReasons] = useState<AttritionReasonOption[] | null>(null);

  useEffect(() => {
    if (!groupId || !open) {
      setEvents(null);
      return;
    }
    setLoading(true);
    api
      .get<GroupEventRow[]>(`/data/groups/${groupId}/events`)
      .then(setEvents)
      .catch(() => setEvents([]))
      .finally(() => setLoading(false));
  }, [groupId, open, tick]);

  useEffect(() => {
    if (evType === "attrition" && !reasons) {
      api
        .get<AttritionReasonOption[]>("/data/attrition-reasons")
        .then(setReasons)
        .catch(() => setReasons([]));
    }
  }, [evType, reasons]);

  const [evErrors, setEvErrors] = useState<Record<string, string>>({});

  const handleAddEvent = async () => {
    if (!groupId) return;
    const errs: Record<string, string> = {};
    if (!evCount || parseInt(evCount, 10) <= 0) errs.count = "К-ть має бути > 0";
    if (!evDate) errs.date = "Обов'язкове поле";
    if (evType === "attrition" && !evReasonId) errs.reason = "Оберіть причину";
    setEvErrors(errs);
    if (Object.keys(errs).length > 0) return;

    setSaving(true);
    try {
      await api.post(`/data/groups/${groupId}/events`, {
        event_type: evType,
        count: parseInt(evCount, 10),
        occurred_on: evDate,
        reason_id: evType === "attrition" ? evReasonId : null,
        note: evNote || null,
      });
      setShowForm(false);
      setEvCount("");
      setEvNote("");
      setEvReasonId(null);
      setEvErrors({});
      setTick((t) => t + 1);
      onChanged?.();
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Не вдалося додати подію";
      setEvErrors({ _server: msg });
    } finally {
      setSaving(false);
    }
  };

  const handleDeleteEvent = async (eventId: number) => {
    if (!groupId) return;
    try {
      await api.delete(`/data/groups/${groupId}/events/${eventId}`);
      setTick((t) => t + 1);
      onChanged?.();
    } catch {
      toast.error("Не вдалося видалити подію");
    }
  };

  return (
    <div className="flex flex-col gap-2">
      {loading && (
        <div className="py-4 text-sm text-muted-foreground">Завантаження...</div>
      )}
      {!loading && events && events.length === 0 && (
        <div className="py-2 text-sm text-muted-foreground">Подій немає</div>
      )}
      {events &&
        events.map((ev) => {
          const info = EVENT_LABELS[ev.event_type] ?? {
            label: ev.event_type,
            icon: "?",
            color: "var(--muted-foreground)",
          };
          return (
            <div
              key={ev.id}
              className="group flex items-center gap-3 rounded-md px-2 py-1.5 hover:bg-accent/40"
            >
              <span className="w-[70px] shrink-0 text-xs tabular-nums text-muted-foreground">
                {ev.occurred_on}
              </span>
              <span
                className="flex h-5 w-5 items-center justify-center rounded-full text-xs"
                style={{ background: info.color + "22", color: info.color }}
              >
                {info.icon}
              </span>
              <span className="flex-1 text-sm">
                {info.label}:{" "}
                <strong className="tabular-nums">{ev.count}</strong>
                {ev.reason_label && (
                  <span className="text-muted-foreground">
                    {" "}
                    ({ev.reason_label})
                  </span>
                )}
              </span>
              {ev.created_by_label && (
                <Link
                  to={`/directory?search=${encodeURIComponent(ev.created_by_label)}`}
                  className="shrink-0 rounded border px-1.5 py-0.5 text-[10px] tracking-wider no-underline transition-colors hover:bg-primary/10"
                  style={{ borderColor: "color-mix(in oklch, var(--primary) 30%, transparent)", color: "var(--primary)", fontFamily: "var(--font-heading)" }}
                  title={`${ev.created_by_label} · ${ev.recorded_at}`}
                  onClick={(e) => e.stopPropagation()}
                >
                  {ev.created_by_label}
                </Link>
              )}
              {ev.source_label && !ev.created_by_label && (
                <span
                  className="shrink-0 rounded border px-1.5 py-0.5 text-[10px] uppercase tracking-wider"
                  style={{ borderColor: "var(--border)", color: "var(--muted-foreground)", fontFamily: "var(--font-heading)" }}
                  title={`Джерело: ${ev.source_label}`}
                >
                  {ev.source_label}
                </span>
              )}
              {ev.note && (
                <span className="max-w-[100px] truncate text-xs text-muted-foreground">
                  {ev.note}
                </span>
              )}
              <Button
                variant="ghost"
                size="icon"
                className="hidden h-5 w-5 text-muted-foreground hover:bg-destructive/10 hover:text-destructive group-hover:flex"
                onClick={() => handleDeleteEvent(ev.id)}
                title="Видалити подію"
              >
                <X className="h-3 w-3" />
              </Button>
            </div>
          );
        })}

      {/* Add event form */}
      {showForm ? (
        <div
          className="mt-1 flex flex-col gap-2 rounded-lg border p-3"
          style={{ borderColor: "var(--border)" }}
        >
          <div className="flex gap-2">
            <div className="flex-1">
              <Select value={evType} onValueChange={setEvType}>
                <SelectTrigger className="h-8 w-full">
                  <SelectValue>
                    {EVENT_TYPE_OPTIONS.find((o) => o.value === evType)?.label ?? evType}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  {EVENT_TYPE_OPTIONS.map((o) => (
                    <SelectItem key={o.value} value={o.value}>
                      {o.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div>
              <Input
                className="h-8 w-20 tabular-nums"
                style={evErrors.count ? { borderColor: "#D9534F" } : undefined}
                type="number"
                min={1}
                placeholder="К-ть"
                value={evCount}
                onChange={(e) => { setEvCount(e.target.value); setEvErrors((p) => { const { count, ...rest } = p; return rest; }); }}
              />
              <FieldError error={evErrors.count} />
            </div>
          </div>
          <div>
            <DateInputUa
              value={evDate}
              onChange={(v) => { setEvDate(v); setEvErrors((p) => { const { date, ...rest } = p; return rest; }); }}
            />
            <FieldError error={evErrors.date} />
          </div>
          {evType === "attrition" && reasons && (
            <div>
              <Select
                value={evReasonId != null ? String(evReasonId) : ""}
                onValueChange={(v) => {
                  setEvReasonId(v ? Number(v) : null);
                  setEvErrors((p) => { const { reason, ...rest } = p; return rest; });
                }}
              >
                <SelectTrigger className="h-8 w-full" style={evErrors.reason ? { borderColor: "#D9534F" } : undefined}>
                  <SelectValue placeholder="Причина вибуття…">
                    {evReasonId != null ? reasons.find((r) => r.id === evReasonId)?.name : "Причина вибуття…"}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  {reasons.map((r) => (
                    <SelectItem key={r.id} value={String(r.id)}>
                      {r.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <FieldError error={evErrors.reason} />
            </div>
          )}
          <FieldError error={evErrors._server} />
          <Input
            className="h-8"
            placeholder="Примітка (необов'язково)"
            value={evNote}
            onChange={(e) => setEvNote(e.target.value)}
          />
          <div className="flex gap-2">
            <Button
              size="sm"
              onClick={handleAddEvent}
              disabled={saving || !evCount}
            >
              {saving ? (
                <Loader2 className="mr-1 h-3 w-3 animate-spin" />
              ) : (
                <Plus className="mr-1 h-3 w-3" />
              )}
              Додати
            </Button>
            <Button
              size="sm"
              variant="outline"
              onClick={() => setShowForm(false)}
            >
              Скасувати
            </Button>
          </div>
        </div>
      ) : (
        <Button
          variant="ghost"
          size="sm"
          className="mt-1 text-muted-foreground hover:text-foreground"
          onClick={() => {
            setShowForm(true);
            setEvDate(todayIso());
          }}
        >
          <Plus className="h-3.5 w-3.5" />
          Додати подію
        </Button>
      )}
    </div>
  );
}

function DetailPanel({
  group,
  open,
  onOpenChange,
  onGroupChanged,
}: {
  group: DataGroupRow | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onGroupChanged?: () => void;
}) {
  if (!group) return null;

  const pct =
    group.planned_count > 0
      ? Math.round((group.in_training_count / group.planned_count) * 100)
      : 0;

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side="right" dockable className="w-full overflow-y-auto sm:max-w-xl lg:max-w-2xl">
        <SheetHeader>
          <SheetTitle>{group.org_label}</SheetTitle>
          <SheetDescription>
            {group.training_kind} · #{group.id}
          </SheetDescription>
        </SheetHeader>

        <div className="flex flex-col gap-5 px-4 pb-6">
          {/* Info fields */}
          <div className="grid grid-cols-2 gap-3">
            <Field icon={FileText} label="ВОС / курс" value={group.vos_label} />
            <Field
              icon={MapPin}
              label="Місце"
              value={
                [group.site_label, group.city_label].filter(Boolean).join(", ") || "—"
              }
            />
            <Field
              icon={CalendarIcon}
              label="Період"
              value={
                group.planned_start && group.planned_end
                  ? `${group.planned_start} — ${group.planned_end}`
                  : group.planned_start || "—"
              }
            />
            <Field icon={Users} label="Організатор" value={group.organizer_label} />
            {group.equipment_text && (
              <Field icon={Layers} label="ОВТ" value={group.equipment_text} />
            )}
            {group.note && (
              <Field icon={Hash} label="Примітка" value={group.note} />
            )}
          </div>

          {/* Count boxes */}
          <div className="grid grid-cols-5 gap-2">
            <CountBox label="План" value={group.planned_count} accent />
            <CountBox label="Прибуло" value={group.arrived_count} />
            <CountBox label="Навч." value={group.in_training_count} />
            <CountBox label="Заверш." value={group.completed_count} />
            <CountBox label="Вибуло" value={group.attrition_count} danger={group.attrition_count > 0} />
          </div>

          {/* Progress bar */}
          {group.planned_count > 0 && (
            <div className="flex items-center gap-2">
              <div
                className="flex-1 overflow-hidden rounded-full"
                style={{ height: 6, background: "var(--muted)" }}
              >
                <div
                  className="h-full rounded-full transition-all duration-500"
                  style={{
                    width: `${Math.min(100, pct)}%`,
                    background: "var(--primary)",
                  }}
                />
              </div>
              <span className="text-xs tabular-nums text-muted-foreground">
                {pct}%
              </span>
            </div>
          )}

          {/* Events timeline */}
          <div>
            <h3
              className="mb-2 text-[11px] font-semibold uppercase tracking-wider"
              style={{ color: "var(--muted-foreground)" }}
            >
              Хронологія подій
            </h3>
            <EventTimeline groupId={group.id} open={open} onChanged={onGroupChanged} />
          </div>
        </div>
      </SheetContent>
    </Sheet>
  );
}

function Field({
  icon: Icon,
  label,
  value,
}: {
  icon: React.ElementType;
  label: string;
  value: string;
}) {
  return (
    <div className="flex items-start gap-2">
      <Icon
        className="mt-0.5 h-3.5 w-3.5 shrink-0"
        style={{ color: "var(--muted-foreground)" }}
      />
      <div className="flex flex-col">
        <span
          className="text-[10px] font-semibold uppercase tracking-wider"
          style={{ color: "var(--muted-foreground)" }}
        >
          {label}
        </span>
        <span className="text-sm">{value || "—"}</span>
      </div>
    </div>
  );
}

function CountBox({
  label,
  value,
  accent,
  danger,
}: {
  label: string;
  value: number;
  accent?: boolean;
  danger?: boolean;
}) {
  return (
    <div
      className="flex flex-col items-center rounded-lg border px-2 py-2"
      style={{
        borderColor: accent
          ? "var(--primary)"
          : danger
            ? "#D9534F44"
            : "var(--border)",
        background: accent
          ? "color-mix(in srgb, var(--primary) 5%, transparent)"
          : danger
            ? "rgba(217,83,79,0.05)"
            : "transparent",
      }}
    >
      <span
        className="text-lg font-bold tabular-nums"
        style={{
          color: accent
            ? "var(--primary)"
            : danger
              ? "#D9534F"
              : "var(--foreground)",
        }}
      >
        {value}
      </span>
      <span
        className="text-[10px] font-semibold uppercase tracking-wider"
        style={{ color: "var(--muted-foreground)" }}
      >
        {label}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Import Excel Section
// ---------------------------------------------------------------------------

type ImportFileKind = "fah" | "bps" | "kvid" | "terminy" | "ivs" | "archive";

const IMPORT_KINDS: { value: ImportFileKind; label: string; desc: string }[] = [
  { value: "fah", label: "Фах", desc: "Фахова підготовка (зведена таблиця)" },
  { value: "bps", label: "БпС", desc: "Бойова підготовка складових (зведена таблиця)" },
  { value: "kvid", label: "КВід", desc: "Укомплектованість командирами відділень" },
  { value: "terminy", label: "Терміни", desc: "Терміни підготовки (БЗВП/фахова/адаптація)" },
  { value: "ivs", label: "ІВС", desc: "Укомплектованість інструкторів + стажування/курси" },
  { value: "archive", label: "Архів ВЧ", desc: "Одноразовий перенос архіву фахової підготовки" },
];

interface ImportIssue {
  row: number;
  sheet: string;
  field: string;
  value: string;
  message: string;
}

interface ImportError {
  message: string;
  issues: ImportIssue[];
}

const FIELD_STYLE: Record<string, { icon: typeof Building2; color: string }> = {
  "Частина": { icon: Building2, color: "border-blue-500/50 bg-blue-500/10 text-blue-400" },
  "Місце": { icon: MapPin, color: "border-emerald-500/50 bg-emerald-500/10 text-emerald-400" },
  "ВОС/Посада": { icon: GraduationCap, color: "border-violet-500/50 bg-violet-500/10 text-violet-400" },
};

function FieldBadge({ field }: { field: string }) {
  const style = FIELD_STYLE[field];
  if (!style) return <Badge variant="outline" className="text-xs">{field}</Badge>;
  const Icon = style.icon;
  return (
    <Badge variant="outline" className={`text-xs ${style.color}`}>
      <Icon className="mr-1 h-3 w-3" />
      {field}
    </Badge>
  );
}

function ImportDropZone({
  fileName,
  onFile,
  disabled,
}: {
  fileName: string | null;
  onFile: (file: File) => void;
  disabled?: boolean;
}) {
  const [dragOver, setDragOver] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  function handleDrop(e: React.DragEvent) {
    e.preventDefault();
    setDragOver(false);
    if (disabled) return;
    const file = e.dataTransfer.files?.[0];
    if (file) onFile(file);
  }

  return (
    <div
      onDragOver={(e) => { e.preventDefault(); if (!disabled) setDragOver(true); }}
      onDragLeave={() => setDragOver(false)}
      onDrop={handleDrop}
      onClick={() => !disabled && inputRef.current?.click()}
      className="cursor-pointer rounded-lg border-2 border-dashed p-8 text-center transition-colors"
      style={{
        borderColor: dragOver ? "#c9a84c" : "rgba(138,133,119,0.3)",
        background: dragOver ? "rgba(201,168,76,0.04)" : "transparent",
        opacity: disabled ? 0.5 : 1,
      }}
    >
      <input
        ref={inputRef}
        type="file"
        accept=".xlsx,.xls"
        onChange={(e) => { const f = e.target.files?.[0]; if (f) onFile(f); }}
        className="hidden"
        disabled={disabled}
      />
      <div className="flex flex-col items-center gap-3">
        <div className="icon-box" style={{ width: 48, height: 48 }}>
          {fileName ? <FileUp className="h-6 w-6" /> : <Upload className="h-6 w-6" />}
        </div>
        {fileName ? (
          <div>
            <p className="text-sm font-medium">{fileName}</p>
            <p className="text-xs text-muted-foreground">Натисніть або перетягніть інший файл</p>
          </div>
        ) : (
          <div>
            <p className="text-sm font-medium">Перетягніть файл сюди</p>
            <p className="text-xs text-muted-foreground">або натисніть для вибору (.xlsx)</p>
          </div>
        )}
      </div>
    </div>
  );
}

function RecentSubmissions() {
  const [subs, setSubs] = useState<AdminSubmissionRow[] | null>(null);

  useEffect(() => {
    api
      .get<AdminSubmissionRow[]>("/submissions/recent")
      .then(setSubs)
      .catch(() => setSubs([]));
  }, []);

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2 text-sm font-semibold">
        <Clock className="h-4 w-4" />
        Останні подання
      </div>
      {subs === null ? (
        <div className="flex flex-col gap-2">
          <Skeleton className="h-8 w-full" />
          <Skeleton className="h-8 w-full" />
        </div>
      ) : subs.length === 0 ? (
        <p className="py-4 text-center text-sm text-muted-foreground">
          Подань ще немає
        </p>
      ) : (
        <div className="overflow-x-auto">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Підрозділ</TableHead>
                <TableHead>Тип</TableHead>
                <TableHead>Статус</TableHead>
                <TableHead>Станом на</TableHead>
                <TableHead>Оновлено</TableHead>
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
                  <TableCell>{s.as_of_date}</TableCell>
                  <TableCell>{s.updated_at}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}
    </div>
  );
}

function ImportSection({ onImported }: { onImported: () => void }) {
  const [open, setOpen] = useState(false);
  const [fileKind, setFileKind] = useState<ImportFileKind>("fah");
  const [fileName, setFileName] = useState<string | null>(null);
  const [file, setFile] = useState<File | null>(null);
  const [uploading, setUploading] = useState(false);
  const [result, setResult] = useState<{ imported: number } | null>(null);
  const [error, setError] = useState<ImportError | null>(null);
  const [downloadingTemplate, setDownloadingTemplate] = useState(false);
  const kindInfo = IMPORT_KINDS.find((k) => k.value === fileKind);

  function handleFile(f: File) {
    setFileName(f.name);
    setFile(f);
    setResult(null);
    setError(null);
  }

  async function handleUpload() {
    if (!file) return;
    setUploading(true);
    setResult(null);
    setError(null);
    try {
      const formData = new FormData();
      formData.append("kind", fileKind);
      formData.append("file", file);
      const res = await fetch("/api/import/upload", {
        method: "POST",
        body: formData,
        credentials: "include",
      });
      if (!res.ok) {
        const body = await res.json().catch(() => ({ error: `Помилка ${res.status}` }));
        setError({
          message: body.error || `Помилка ${res.status}`,
          issues: Array.isArray(body.issues) ? body.issues : [],
        });
        toast.error(body.error || `Помилка ${res.status}`);
        return;
      }
      const data = await res.json();
      setResult({ imported: data.imported ?? 0 });
      setFile(null);
      setFileName(null);
      toast.success(`Імпортовано ${data.imported ?? 0} записів`);
      onImported();
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Невідома помилка";
      setError({ message: msg, issues: [] });
      toast.error(msg);
    } finally {
      setUploading(false);
    }
  }

  async function handleDownloadTemplate() {
    setDownloadingTemplate(true);
    try {
      const res = await fetch("/api/import/template", {
        credentials: "include",
      });
      if (!res.ok) throw new Error(`Помилка ${res.status}`);
      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "import-template.xlsx";
      a.click();
      URL.revokeObjectURL(url);
    } catch {
      toast.error("Не вдалося завантажити зразок");
    } finally {
      setDownloadingTemplate(false);
    }
  }

  return (
    <div
      className="rounded-lg border"
      style={{ borderColor: "var(--border)" }}
    >
      <Button
        variant="ghost"
        className="flex w-full justify-start gap-2 px-4 py-3 text-sm font-semibold"
        onClick={() => setOpen(!open)}
      >
        {open ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
        <Upload className="h-4 w-4 text-primary" />
        Імпорт з файлу
      </Button>
      {open && (
        <div className="flex flex-col gap-5 border-t px-4 py-4" style={{ borderColor: "var(--border)" }}>
          <div className="flex flex-col gap-1.5">
            <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
              Тип файлу
            </span>
            <Select
              value={fileKind}
              onValueChange={(v) => {
                setFileKind(v as ImportFileKind);
                setFileName(null);
                setFile(null);
                setResult(null);
                setError(null);
              }}
            >
              <SelectTrigger className="w-full max-w-sm">
                <SelectValue placeholder="Оберіть тип">
                  {kindInfo?.label}
                </SelectValue>
              </SelectTrigger>
              <SelectContent>
                {IMPORT_KINDS.map((k) => (
                  <SelectItem key={k.value} value={k.value}>{k.label}</SelectItem>
                ))}
              </SelectContent>
            </Select>
            {kindInfo && (
              <p className="text-xs text-muted-foreground">{kindInfo.desc}</p>
            )}
          </div>

          <ImportDropZone fileName={fileName} onFile={handleFile} disabled={uploading} />

          {error && (
            <div className="flex flex-col gap-3">
              <Alert variant="destructive">
                <FileWarning className="h-4 w-4" />
                <AlertDescription className="flex items-center justify-between">
                  <span>{error.message}</span>
                  {error.issues.length > 0 && (
                    <Badge variant="destructive" className="ml-2 tabular-nums badge-pulse">
                      {error.issues.length} {error.issues.length === 1 ? "проблема" : error.issues.length < 5 ? "проблеми" : "проблем"}
                    </Badge>
                  )}
                </AlertDescription>
              </Alert>
              {error.issues.length > 0 && (
                <div className="max-h-80 overflow-auto rounded-md border border-destructive/30">
                  <Table>
                    <TableHeader>
                      <TableRow className="border-b-destructive/30 bg-destructive/5">
                        <TableHead className="w-20">Рядок</TableHead>
                        <TableHead className="w-28">Аркуш</TableHead>
                        <TableHead className="w-32">Поле</TableHead>
                        <TableHead>Значення</TableHead>
                        <TableHead>Помилка</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {error.issues.map((issue, i) => (
                        <TableRow key={i} className="border-b-destructive/10 hover:bg-destructive/5">
                          <TableCell>
                            {issue.row ? (
                              <Badge variant="outline" className="tabular-nums border-amber-500/50 bg-amber-500/10 text-amber-400">
                                #{issue.row}
                              </Badge>
                            ) : "—"}
                          </TableCell>
                          <TableCell className="text-xs text-muted-foreground">{issue.sheet || "—"}</TableCell>
                          <TableCell>
                            <FieldBadge field={issue.field} />
                          </TableCell>
                          <TableCell className="max-w-[200px] truncate font-mono text-xs text-amber-300/80">
                            {issue.value || "—"}
                          </TableCell>
                          <TableCell className="text-sm text-destructive">{issue.message}</TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </div>
              )}
            </div>
          )}

          {result && (
            <Alert>
              <CheckCircle2 className="h-4 w-4" />
              <AlertDescription>Імпортовано {result.imported} записів</AlertDescription>
            </Alert>
          )}

          <div className="flex items-center gap-3">
            <Button
              onClick={handleUpload}
              disabled={!file || uploading}
            >
              {uploading ? (
                <>
                  <Loader2 className="mr-1.5 h-4 w-4 animate-spin" />
                  Завантаження...
                </>
              ) : (
                <>
                  <Upload className="mr-1.5 h-4 w-4" />
                  Імпортувати
                </>
              )}
            </Button>
            <Button
              variant="outline"
              onClick={handleDownloadTemplate}
              disabled={downloadingTemplate}
            >
              {downloadingTemplate ? (
                <Loader2 className="mr-1.5 h-4 w-4 animate-spin" />
              ) : (
                <Download className="mr-1.5 h-4 w-4" />
              )}
              Завантажити зразок
            </Button>
          </div>

          <RecentSubmissions />
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Create Group Dialog
// ---------------------------------------------------------------------------

interface TrainingKindOpt {
  id: number;
  code: string;
  name: string;
}

function CreateGroupDialog({
  open,
  onOpenChange,
  onCreated,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreated: () => void;
}) {
  const [kinds, setKinds] = useState<TrainingKindOpt[] | null>(null);
  const [saving, setSaving] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [serverError, setServerError] = useState<string | null>(null);
  const [duplicates, setDuplicates] = useState<{ id: number; vos_label: string; site_label: string; planned_count: number }[] | null>(null);

  // Dictionaries for optional fields
  const [dicts, setDicts] = useState<{
    vos: { id: number; label: string }[];
    positions: { id: number; label: string }[];
    courses: { id: number; label: string }[];
    bzvp_programs: { id: number; label: string }[];
    equipment: { id: number; label: string }[];
  } | null>(null);

  // Optional fields
  const [vosId, setVosId] = useState<number | null>(null);
  const [vosLabel, setVosLabel] = useState("");
  const [vosCmdOpen, setVosCmdOpen] = useState(false);
  const [positionId, setPositionId] = useState<number | null>(null);
  const [courseId, setCourseId] = useState<number | null>(null);
  const [bzvpProgramId, setBzvpProgramId] = useState<number | null>(null);
  const [equipmentText, setEquipmentText] = useState("");
  const [organizerOrgId, setOrganizerOrgId] = useState<number | null>(null);
  const [organizerLabel, setOrganizerLabel] = useState("");
  const [organizerQuery, setOrganizerQuery] = useState("");
  const [organizerResults, setOrganizerResults] = useState<{ org_id: number; label: string }[]>([]);
  const [organizerCmdOpen, setOrganizerCmdOpen] = useState(false);
  const [basisDocNumber, setBasisDocNumber] = useState("");
  const [basisDocDate, setBasisDocDate] = useState("");

  // Form fields
  const [orgQuery, setOrgQuery] = useState("");
  const [orgId, setOrgId] = useState<number | null>(null);
  const [orgLabel, setOrgLabel] = useState("");
  const [kindId, setKindId] = useState<number>(1);
  const [venueType, setVenueType] = useState<string>("");
  const [venueQuery, setVenueQuery] = useState("");
  const [venueId, setVenueId] = useState<number | null>(null);
  const [venueLabel, setVenueLabel] = useState("");
  const [venueCityId, setVenueCityId] = useState<number | null>(null);
  const [venueCityLabel, setVenueCityLabel] = useState("");
  const [cityQuery, setCityQuery] = useState("");
  const [cityId, setCityId] = useState<number | null>(null);
  const [cityLabel, setCityLabel] = useState("");
  const [plannedStart, setPlannedStart] = useState("");
  const [plannedEnd, setPlannedEnd] = useState("");
  const [plannedCount, setPlannedCount] = useState("");
  const [arrivedCount, setArrivedCount] = useState("");
  const [note, setNote] = useState("");

  // Org/venue/city search
  const [orgResults, setOrgResults] = useState<{ org_id: number; label: string }[]>([]);
  const [venueResults, setVenueResults] = useState<VenueSearchRow[]>([]);
  const [cityResults, setCityResults] = useState<CityRow[]>([]);
  const [orgCmdOpen, setOrgCmdOpen] = useState(false);
  const [venueCmdOpen, setVenueCmdOpen] = useState(false);
  const [cityCmdOpen, setCityCmdOpen] = useState(false);

  useEffect(() => {
    if (open && !kinds) {
      api.get<TrainingKindOpt[]>("/data/training-kinds").then(setKinds).catch(() => {});
    }
    if (open && !dicts) {
      api.get<typeof dicts>("/admin/dictionaries").then(setDicts).catch(() => {});
    }
  }, [open, kinds, dicts]);

  useEffect(() => {
    if (organizerQuery.length < 2) {
      setOrganizerResults([]);
      return;
    }
    const t = setTimeout(() => {
      api
        .get<{ org_id: number; label: string }[]>(
          `/orgs/search?q=${encodeURIComponent(organizerQuery)}&limit=5&scope=visible`,
        )
        .then(setOrganizerResults)
        .catch(() => setOrganizerResults([]));
    }, 300);
    return () => clearTimeout(t);
  }, [organizerQuery]);

  useEffect(() => {
    if (orgQuery.length < 2) {
      setOrgResults([]);
      return;
    }
    const t = setTimeout(() => {
      api
        .get<{ org_id: number; label: string }[]>(
          `/orgs/search?q=${encodeURIComponent(orgQuery)}&limit=5&scope=visible`,
        )
        .then(setOrgResults)
        .catch(() => setOrgResults([]));
    }, 300);
    return () => clearTimeout(t);
  }, [orgQuery]);

  useEffect(() => {
    if (venueQuery.length < 2) {
      setVenueResults([]);
      return;
    }
    const t = setTimeout(() => {
      api
        .get<VenueSearchRow[]>(
          `/data/training-sites?q=${encodeURIComponent(venueQuery)}&limit=8`,
        )
        .then((res) => {
          const kind = venueType === "vvnz" ? "vvnz" : "training_center";
          setVenueResults(res.filter((v) => v.kind === kind));
        })
        .catch(() => setVenueResults([]));
    }, 300);
    return () => clearTimeout(t);
  }, [venueQuery, venueType]);

  useEffect(() => {
    if (cityQuery.length < 2) {
      setCityResults([]);
      return;
    }
    const t = setTimeout(() => {
      api
        .get<CityRow[]>(
          `/data/cities?q=${encodeURIComponent(cityQuery)}&limit=8`,
        )
        .then(setCityResults)
        .catch(() => setCityResults([]));
    }, 300);
    return () => clearTimeout(t);
  }, [cityQuery]);

  function validate(): Record<string, string> {
    const e: Record<string, string> = {};
    if (!orgId) e.org = "Оберіть підрозділ";
    if (!venueType) e.venue_type = "Оберіть тип місця";
    if (venueType === "training_center" || venueType === "vvnz") {
      if (!venueId) e.venue = "Оберіть місце проведення";
    }
    if (venueType === "unit_base" && !cityId) {
      e.city = "Оберіть місто";
    }
    if (!plannedStart) e.start = "Обов'язкове поле";
    if (!plannedEnd) e.end = "Обов'язкове поле";
    if (plannedStart && plannedEnd && plannedEnd < plannedStart) {
      e.end = "Кінець раніше початку";
    }
    const cntErr = validateCount(plannedCount, true);
    if (cntErr) e.count = cntErr;
    else if (parseInt(plannedCount, 10) <= 0) e.count = "Має бути > 0";
    const arrErr = validateCount(arrivedCount, false);
    if (arrErr) e.arrived = arrErr;
    return e;
  }

  const isFormValid = Boolean(
    orgId
    && venueType
    && (venueType === "unit_base" ? cityId : venueId)
    && plannedStart
    && plannedEnd
    && plannedEnd >= plannedStart
    && plannedCount
    && parseInt(plannedCount, 10) > 0
    && !validateCount(arrivedCount, false)
  );

  type CreateRes = { id?: number; warning?: string; existing?: { id: number; vos_label: string; site_label: string; planned_count: number }[] };

  const submitGroup = async (force = false) => {
    const resolvedCityId = venueType === "unit_base" ? cityId : venueCityId;
    return api.post<CreateRes>("/data/groups", {
      sender_org_id: orgId,
      training_kind_id: kindId,
      venue_type: venueType || undefined,
      training_venue_id: venueType === "unit_base" ? undefined : venueId,
      city_id: resolvedCityId,
      planned_start: plannedStart,
      planned_end: plannedEnd,
      planned_count: parseInt(plannedCount, 10),
      arrived_count: arrivedCount ? parseInt(arrivedCount, 10) : 0,
      note: note || undefined,
      vos_id: vosId || undefined,
      position_id: positionId || undefined,
      course_id: courseId || undefined,
      bzvp_program_id: bzvpProgramId || undefined,
      equipment_text: equipmentText || undefined,
      organizer_org_id: organizerOrgId || undefined,
      basis_doc_number: basisDocNumber || undefined,
      basis_doc_date: basisDocDate || undefined,
      force,
    });
  };

  const resetForm = () => {
    setOrgQuery("");
    setOrgId(null);
    setOrgLabel("");
    setVenueType("");
    setVenueQuery("");
    setVenueId(null);
    setVenueLabel("");
    setVenueCityId(null);
    setVenueCityLabel("");
    setCityQuery("");
    setCityId(null);
    setCityLabel("");
    setPlannedStart("");
    setPlannedEnd("");
    setPlannedCount("");
    setArrivedCount("");
    setNote("");
    setVosId(null);
    setVosLabel("");
    setPositionId(null);
    setCourseId(null);
    setBzvpProgramId(null);
    setEquipmentText("");
    setOrganizerOrgId(null);
    setOrganizerLabel("");
    setOrganizerQuery("");
    setOrganizerResults([]);
    setBasisDocNumber("");
    setBasisDocDate("");
    setErrors({});
    setDuplicates(null);
  };

  const handleCreate = async () => {
    const errs = validate();
    setErrors(errs);
    setServerError(null);
    if (Object.keys(errs).length > 0) return;

    setSaving(true);
    try {
      const res = await submitGroup(false);
      if (res.warning && res.existing) {
        setDuplicates(res.existing);
        setSaving(false);
        return;
      }
      onOpenChange(false);
      onCreated();
      resetForm();
    } catch (err) {
      setServerError(err instanceof Error ? err.message : "Помилка створення");
    } finally {
      setSaving(false);
    }
  };

  const handleForceCreate = async () => {
    setSaving(true);
    setServerError(null);
    try {
      await submitGroup(true);
      onOpenChange(false);
      onCreated();
      resetForm();
    } catch (err) {
      setServerError(err instanceof Error ? err.message : "Помилка створення");
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>Нова група підготовки</DialogTitle>
          <DialogDescription>
            Поля з * обов'язкові. Решта — за наявності.
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-5">
          {/* Org + Kind — side by side */}
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-[1fr_180px]">
            <div className="flex flex-col gap-1.5">
              <Label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                <Building2 className="mr-1 inline h-3 w-3" />
                Підрозділ <span style={{ color: "#D9534F" }}>*</span>
              </Label>
              <Popover open={orgCmdOpen} onOpenChange={setOrgCmdOpen}>
                <PopoverTrigger
                  render={<Button variant="outline" className="w-full justify-start text-left font-normal" />}
                >
                  <Building2 className="mr-2 h-4 w-4 shrink-0 text-muted-foreground" />
                  {orgId ? (
                    <span className="flex-1 truncate">{orgLabel}</span>
                  ) : (
                    <span className="flex-1 text-muted-foreground">Пошук підрозділу…</span>
                  )}
                  {orgId && (
                    <span
                      className="ml-auto rounded-md p-0.5 text-muted-foreground hover:text-foreground"
                      onClick={(e) => { e.stopPropagation(); setOrgId(null); setOrgLabel(""); setOrgQuery(""); }}
                    >
                      <X className="h-3.5 w-3.5" />
                    </span>
                  )}
                </PopoverTrigger>
                <PopoverContent className="w-[--anchor-width] p-0" align="start">
                  <Command shouldFilter={false}>
                    <CommandInput
                      placeholder="Почніть вводити назву…"
                      value={orgQuery}
                      onValueChange={setOrgQuery}
                    />
                    <CommandList>
                      {orgQuery.length >= 2 && orgResults.length === 0 && (
                        <CommandEmpty>Не знайдено</CommandEmpty>
                      )}
                      {orgQuery.length < 2 && (
                        <CommandEmpty>Введіть мін. 2 символи</CommandEmpty>
                      )}
                      <CommandGroup>
                        {orgResults.map((r) => (
                          <CommandItem
                            key={r.org_id}
                            value={String(r.org_id)}
                            onSelect={() => {
                              setOrgId(r.org_id);
                              setOrgLabel(r.label);
                              setOrgQuery("");
                              setOrgResults([]);
                              setOrgCmdOpen(false);
                            }}
                          >
                            <Building2 className="h-3.5 w-3.5 text-muted-foreground" />
                            {r.label}
                          </CommandItem>
                        ))}
                      </CommandGroup>
                    </CommandList>
                  </Command>
                </PopoverContent>
              </Popover>
              <FieldError error={errors.org} />
            </div>

            <div className="flex flex-col gap-1.5">
              <Label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                <FileText className="mr-1 inline h-3 w-3" />
                Вид <span style={{ color: "#D9534F" }}>*</span>
              </Label>
              <Select
                value={String(kindId)}
                onValueChange={(v) => setKindId(Number(v))}
              >
                <SelectTrigger className="w-full">
                  <SelectValue>
                    {kinds?.find((k) => k.id === kindId)?.name ?? "…"}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  {(kinds ?? []).map((k) => (
                    <SelectItem key={k.id} value={String(k.id)}>
                      {k.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          {/* ВОС autocomplete */}
          <div className="flex flex-col gap-1.5">
            <Label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              <GraduationCap className="mr-1 inline h-3 w-3" />
              ВОС
            </Label>
            <Popover open={vosCmdOpen} onOpenChange={setVosCmdOpen}>
              <PopoverTrigger
                render={<Button variant="outline" className="w-full justify-start text-left font-normal" />}
              >
                <GraduationCap className="mr-2 h-4 w-4 shrink-0 text-muted-foreground" />
                {vosId ? (
                  <span className="flex-1 truncate">{vosLabel}</span>
                ) : (
                  <span className="flex-1 text-muted-foreground">Оберіть ВОС…</span>
                )}
                {vosId && (
                  <span
                    className="ml-auto rounded-md p-0.5 text-muted-foreground hover:text-foreground"
                    onClick={(e) => { e.stopPropagation(); setVosId(null); setVosLabel(""); }}
                  >
                    <X className="h-3.5 w-3.5" />
                  </span>
                )}
              </PopoverTrigger>
              <PopoverContent className="w-[--anchor-width] p-0" align="start">
                <Command>
                  <CommandInput placeholder="Пошук за кодом або назвою…" />
                  <CommandList>
                    <CommandEmpty>Не знайдено</CommandEmpty>
                    <CommandGroup>
                      {(dicts?.vos ?? []).map((v) => (
                        <CommandItem
                          key={v.id}
                          value={v.label}
                          onSelect={() => {
                            setVosId(v.id);
                            setVosLabel(v.label);
                            setVosCmdOpen(false);
                          }}
                        >
                          <GraduationCap className="h-3.5 w-3.5 text-muted-foreground" />
                          {v.label}
                        </CommandItem>
                      ))}
                    </CommandGroup>
                  </CommandList>
                </Command>
              </PopoverContent>
            </Popover>
          </div>

          {/* Посада + Курс */}
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
            <div className="flex flex-col gap-1.5">
              <Label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Посада
              </Label>
              <Select
                value={positionId ? String(positionId) : ""}
                onValueChange={(v) => setPositionId(v ? Number(v) : null)}
              >
                <SelectTrigger className="w-full">
                  <SelectValue placeholder="—" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="">—</SelectItem>
                  {(dicts?.positions ?? []).map((p) => (
                    <SelectItem key={p.id} value={String(p.id)}>
                      {p.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="flex flex-col gap-1.5">
              <Label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Курс
              </Label>
              <Select
                value={courseId ? String(courseId) : ""}
                onValueChange={(v) => setCourseId(v ? Number(v) : null)}
              >
                <SelectTrigger className="w-full">
                  <SelectValue placeholder="—" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="">—</SelectItem>
                  {(dicts?.courses ?? []).map((c) => (
                    <SelectItem key={c.id} value={String(c.id)}>
                      {c.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          {/* БЗВП program — only for bzvp kind */}
          {kinds?.find((k) => k.id === kindId)?.code === "bzvp" && (
            <div className="flex flex-col gap-1.5">
              <Label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Програма БЗВП
              </Label>
              <Select
                value={bzvpProgramId ? String(bzvpProgramId) : ""}
                onValueChange={(v) => setBzvpProgramId(v ? Number(v) : null)}
              >
                <SelectTrigger className="w-full">
                  <SelectValue placeholder="—" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="">—</SelectItem>
                  {(dicts?.bzvp_programs ?? []).map((b) => (
                    <SelectItem key={b.id} value={String(b.id)}>
                      {b.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}

          {/* ОВТ (equipment) */}
          <div className="flex flex-col gap-1.5">
            <Label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              ОВТ (озброєння)
            </Label>
            <Input
              placeholder="Наприклад: Darts, Switchblade…"
              value={equipmentText}
              onChange={(e) => setEquipmentText(e.target.value)}
            />
          </div>

          {/* Організатор */}
          <div className="flex flex-col gap-1.5">
            <Label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              <Building2 className="mr-1 inline h-3 w-3" />
              Організатор
            </Label>
            <Popover open={organizerCmdOpen} onOpenChange={setOrganizerCmdOpen}>
              <PopoverTrigger
                render={<Button variant="outline" className="w-full justify-start text-left font-normal" />}
              >
                <Building2 className="mr-2 h-4 w-4 shrink-0 text-muted-foreground" />
                {organizerOrgId ? (
                  <span className="flex-1 truncate">{organizerLabel}</span>
                ) : (
                  <span className="flex-1 text-muted-foreground">Пошук організатора…</span>
                )}
                {organizerOrgId && (
                  <span
                    className="ml-auto rounded-md p-0.5 text-muted-foreground hover:text-foreground"
                    onClick={(e) => { e.stopPropagation(); setOrganizerOrgId(null); setOrganizerLabel(""); setOrganizerQuery(""); }}
                  >
                    <X className="h-3.5 w-3.5" />
                  </span>
                )}
              </PopoverTrigger>
              <PopoverContent className="w-[--anchor-width] p-0" align="start">
                <Command shouldFilter={false}>
                  <CommandInput
                    placeholder="Почніть вводити назву…"
                    value={organizerQuery}
                    onValueChange={setOrganizerQuery}
                  />
                  <CommandList>
                    {organizerQuery.length >= 2 && organizerResults.length === 0 && (
                      <CommandEmpty>Не знайдено</CommandEmpty>
                    )}
                    {organizerQuery.length < 2 && (
                      <CommandEmpty>Введіть мін. 2 символи</CommandEmpty>
                    )}
                    <CommandGroup>
                      {organizerResults.map((r) => (
                        <CommandItem
                          key={r.org_id}
                          value={String(r.org_id)}
                          onSelect={() => {
                            setOrganizerOrgId(r.org_id);
                            setOrganizerLabel(r.label);
                            setOrganizerQuery("");
                            setOrganizerResults([]);
                            setOrganizerCmdOpen(false);
                          }}
                        >
                          <Building2 className="h-3.5 w-3.5 text-muted-foreground" />
                          {r.label}
                        </CommandItem>
                      ))}
                    </CommandGroup>
                  </CommandList>
                </Command>
              </PopoverContent>
            </Popover>
          </div>

          {/* Підстава (номер + дата) */}
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-[1fr_160px]">
            <div className="flex flex-col gap-1.5">
              <Label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                <FileText className="mr-1 inline h-3 w-3" />
                Підстава (номер)
              </Label>
              <Input
                placeholder="Номер документа"
                value={basisDocNumber}
                onChange={(e) => setBasisDocNumber(e.target.value)}
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Дата
              </Label>
              <Input
                type="date"
                value={basisDocDate}
                onChange={(e) => setBasisDocDate(e.target.value)}
              />
            </div>
          </div>

          {/* Venue + Period — grouped */}
          <div
            className="flex flex-col gap-3 rounded-lg border p-3"
            style={{ borderColor: "color-mix(in srgb, var(--border) 60%, transparent)" }}
          >
            {/* Venue type selector */}
            <div className="flex flex-col gap-1.5">
              <Label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                <MapPin className="mr-1 inline h-3 w-3" />
                Тип місця <span style={{ color: "#D9534F" }}>*</span>
              </Label>
              <Select
                value={venueType}
                onValueChange={(v) => {
                  setVenueType(v);
                  setVenueId(null);
                  setVenueLabel("");
                  setVenueQuery("");
                  setVenueCityId(null);
                  setVenueCityLabel("");
                  setCityId(null);
                  setCityLabel("");
                  setCityQuery("");
                  setErrors((p) => { const { venue_type, venue, city, ...rest } = p; return rest; });
                }}
              >
                <SelectTrigger className="w-full">
                  <SelectValue>
                    {venueType === "training_center"
                      ? "Навчальний центр"
                      : venueType === "vvnz"
                        ? "ВВНЗ"
                        : venueType === "unit_base"
                          ? "На базі в/ч"
                          : "Оберіть тип…"}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="training_center">Навчальний центр</SelectItem>
                  <SelectItem value="vvnz">ВВНЗ</SelectItem>
                  <SelectItem value="unit_base">На базі в/ч</SelectItem>
                </SelectContent>
              </Select>
              <FieldError error={errors.venue_type} />
            </div>

            {/* Venue autocomplete (for training_center / vvnz) */}
            {(venueType === "training_center" || venueType === "vvnz") && (
              <div className="flex flex-col gap-1.5">
                <Label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                  <MapPin className="mr-1 inline h-3 w-3" />
                  {venueType === "vvnz" ? "ВВНЗ" : "Навчальний центр"} <span style={{ color: "#D9534F" }}>*</span>
                </Label>
                <Popover open={venueCmdOpen} onOpenChange={setVenueCmdOpen}>
                  <PopoverTrigger
                    render={<Button variant="outline" className="w-full justify-start text-left font-normal" />}
                  >
                    <MapPin className="mr-2 h-4 w-4 shrink-0 text-muted-foreground" />
                    {venueId ? (
                      <span className="flex-1 truncate">{venueLabel}</span>
                    ) : (
                      <span className="flex-1 text-muted-foreground">Пошук…</span>
                    )}
                    {venueId && (
                      <span
                        className="ml-auto rounded-md p-0.5 text-muted-foreground hover:text-foreground"
                        onClick={(e) => {
                          e.stopPropagation();
                          setVenueId(null);
                          setVenueLabel("");
                          setVenueQuery("");
                          setVenueCityId(null);
                          setVenueCityLabel("");
                        }}
                      >
                        <X className="h-3.5 w-3.5" />
                      </span>
                    )}
                  </PopoverTrigger>
                  <PopoverContent className="w-[--anchor-width] p-0" align="start">
                    <Command shouldFilter={false}>
                      <CommandInput
                        placeholder="Почніть вводити назву…"
                        value={venueQuery}
                        onValueChange={setVenueQuery}
                      />
                      <CommandList>
                        {venueQuery.length >= 2 && venueResults.length === 0 && (
                          <CommandEmpty>Не знайдено</CommandEmpty>
                        )}
                        {venueQuery.length < 2 && (
                          <CommandEmpty>Введіть мін. 2 символи</CommandEmpty>
                        )}
                        <CommandGroup>
                          {venueResults.map((r) => (
                            <CommandItem
                              key={r.id}
                              value={String(r.id)}
                              onSelect={() => {
                                setVenueId(r.id);
                                setVenueLabel(`${r.name} (${r.city_name})`);
                                setVenueCityId(r.city_id);
                                setVenueCityLabel(r.city_name);
                                setVenueQuery("");
                                setVenueResults([]);
                                setVenueCmdOpen(false);
                              }}
                            >
                              <MapPin className="h-3.5 w-3.5 text-muted-foreground" />
                              <span>{r.name}</span>
                              <span className="ml-auto text-xs text-muted-foreground">{r.city_name}</span>
                            </CommandItem>
                          ))}
                        </CommandGroup>
                      </CommandList>
                    </Command>
                  </PopoverContent>
                </Popover>
                {venueCityLabel && (
                  <span className="text-xs text-muted-foreground">м. {venueCityLabel}</span>
                )}
                <FieldError error={errors.venue} />
              </div>
            )}

            {/* City autocomplete (for unit_base) */}
            {venueType === "unit_base" && (
              <div className="flex flex-col gap-1.5">
                <Label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                  <MapPin className="mr-1 inline h-3 w-3" />
                  Місто <span style={{ color: "#D9534F" }}>*</span>
                </Label>
                <Popover open={cityCmdOpen} onOpenChange={setCityCmdOpen}>
                  <PopoverTrigger
                    render={<Button variant="outline" className="w-full justify-start text-left font-normal" />}
                  >
                    <MapPin className="mr-2 h-4 w-4 shrink-0 text-muted-foreground" />
                    {cityId ? (
                      <span className="flex-1 truncate">{cityLabel}</span>
                    ) : (
                      <span className="flex-1 text-muted-foreground">Пошук міста…</span>
                    )}
                    {cityId && (
                      <span
                        className="ml-auto rounded-md p-0.5 text-muted-foreground hover:text-foreground"
                        onClick={(e) => { e.stopPropagation(); setCityId(null); setCityLabel(""); setCityQuery(""); }}
                      >
                        <X className="h-3.5 w-3.5" />
                      </span>
                    )}
                  </PopoverTrigger>
                  <PopoverContent className="w-[--anchor-width] p-0" align="start">
                    <Command shouldFilter={false}>
                      <CommandInput
                        placeholder="Почніть вводити місто…"
                        value={cityQuery}
                        onValueChange={setCityQuery}
                      />
                      <CommandList>
                        {cityQuery.length >= 2 && cityResults.length === 0 && (
                          <CommandEmpty>Не знайдено</CommandEmpty>
                        )}
                        {cityQuery.length < 2 && (
                          <CommandEmpty>Введіть мін. 2 символи</CommandEmpty>
                        )}
                        <CommandGroup>
                          {cityResults.map((c) => (
                            <CommandItem
                              key={c.id}
                              value={String(c.id)}
                              onSelect={() => {
                                setCityId(c.id);
                                setCityLabel(c.name);
                                setCityQuery("");
                                setCityResults([]);
                                setCityCmdOpen(false);
                              }}
                            >
                              <MapPin className="h-3.5 w-3.5 text-muted-foreground" />
                              {c.name}
                            </CommandItem>
                          ))}
                        </CommandGroup>
                      </CommandList>
                    </Command>
                  </PopoverContent>
                </Popover>
                <FieldError error={errors.city} />
              </div>
            )}

            <div className="flex flex-col gap-1.5">
              <Label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                <CalendarIcon className="mr-1 inline h-3 w-3" />
                Період <span style={{ color: "#D9534F" }}>*</span>
              </Label>
              <Popover>
                <PopoverTrigger
                  render={<Button variant="outline" className="w-full justify-start text-left font-normal" />}
                >
                  <CalendarIcon className="mr-2 h-4 w-4 text-muted-foreground" />
                  {plannedStart ? (
                    plannedEnd ? (
                      <>
                        {format(parse(plannedStart, "yyyy-MM-dd", new Date()), "dd.MM.yyyy")}
                        {" — "}
                        {format(parse(plannedEnd, "yyyy-MM-dd", new Date()), "dd.MM.yyyy")}
                      </>
                    ) : (
                      format(parse(plannedStart, "yyyy-MM-dd", new Date()), "dd.MM.yyyy")
                    )
                  ) : (
                    <span className="text-muted-foreground">Оберіть період</span>
                  )}
                </PopoverTrigger>
                <PopoverContent className="w-auto p-0" align="start">
                  <Calendar
                    mode="range"
                    locale={uk}
                    selected={{
                      from: plannedStart ? parse(plannedStart, "yyyy-MM-dd", new Date()) : undefined,
                      to: plannedEnd ? parse(plannedEnd, "yyyy-MM-dd", new Date()) : undefined,
                    }}
                    onSelect={(range) => {
                      setPlannedStart(range?.from ? format(range.from, "yyyy-MM-dd") : "");
                      setPlannedEnd(range?.to ? format(range.to, "yyyy-MM-dd") : "");
                      setErrors((p) => { const { start, end, ...rest } = p; return rest; });
                    }}
                    numberOfMonths={2}
                  />
                </PopoverContent>
              </Popover>
              {(errors.start || errors.end) && (
                <div className="flex gap-2">
                  {errors.start && <FieldError error={errors.start} />}
                  {errors.end && <FieldError error={errors.end} />}
                </div>
              )}
            </div>
          </div>

          {/* Counts + Note */}
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
            <div className="flex flex-col gap-1.5">
              <Label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                <Hash className="mr-1 inline h-3 w-3" />
                План (к-ть) <span style={{ color: "#D9534F" }}>*</span>
              </Label>
              <Input
                type="number"
                min={1}
                placeholder="0"
                value={plannedCount}
                onChange={(e) => { setPlannedCount(e.target.value); setErrors((p) => { const { count, ...rest } = p; return rest; }); }}
              />
              <FieldError error={errors.count} />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                <Users className="mr-1 inline h-3 w-3" />
                Прибуло (к-ть)
              </Label>
              <Input
                type="number"
                min={0}
                placeholder="0"
                value={arrivedCount}
                onChange={(e) => { setArrivedCount(e.target.value); setErrors((p) => { const { arrived, ...rest } = p; return rest; }); }}
              />
              <FieldError error={errors.arrived} />
            </div>
            <div className="col-span-2 flex flex-col gap-1.5 sm:col-span-1">
              <Label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Примітка
              </Label>
              <Textarea
                placeholder="Необов'язково"
                value={note}
                onChange={(e) => setNote(e.target.value)}
                rows={2}
                className="resize-none"
              />
            </div>
          </div>
        </div>

        {serverError && (
          <p className="text-sm" style={{ color: "#D9534F" }}>{serverError}</p>
        )}

        {duplicates && (
          <div className="rounded-lg border p-3" style={{ borderColor: "var(--warning)", background: "color-mix(in srgb, var(--warning) 8%, transparent)" }}>
            <p className="mb-2 text-sm font-semibold flex items-center gap-1.5">
              <AlertTriangle className="h-4 w-4" style={{ color: "var(--warning)" }} />
              Схожі заходи вже існують
            </p>
            <div className="space-y-1 text-sm">
              {duplicates.map((d) => (
                <div key={d.id} className="flex items-center gap-2">
                  <span className="font-mono text-xs text-muted-foreground">#{d.id}</span>
                  <span>{d.vos_label || "—"}</span>
                  <span className="text-muted-foreground">{d.site_label}</span>
                  <span className="ml-auto">{d.planned_count} осіб</span>
                </div>
              ))}
            </div>
            <div className="mt-3 flex gap-2">
              <Button variant="outline" size="sm" onClick={() => setDuplicates(null)}>
                Повернутись
              </Button>
              <Button size="sm" onClick={handleForceCreate} disabled={saving}>
                {saving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                Все одно створити
              </Button>
            </div>
          </div>
        )}

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Скасувати
          </Button>
          <Button onClick={handleCreate} disabled={saving || !isFormValid}>
            {saving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            <Plus className="mr-2 h-4 w-4" />
            Створити
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ---------------------------------------------------------------------------
// Main page
// ---------------------------------------------------------------------------

export function DataWorkspacePage() {
  const { isAdmin } = useAuth();

  // Data state
  const [groups, setGroups] = useState<DataGroupRow[] | null>(null);
  const [loading, setLoading] = useState(true);

  // Table state
  const [sorting, setSorting] = useState<SortingState>([
    { id: "planned_start", desc: true },
  ]);
  const [globalFilter, setGlobalFilter] = useState("");
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([]);
  const [grouping, setGrouping] = useState<GroupingState>([]);

  // UI state
  const [selectedGroup, setSelectedGroup] = useState<DataGroupRow | null>(null);
  const [detailOpen, setDetailOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<DataGroupRow | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [showCreateDialog, setShowCreateDialog] = useState(false);

  // Kind filter (quick filter chips)
  const [kindFilter, setKindFilter] = useState<string | null>(null);

  // Grouping by
  const [groupBy, setGroupBy] = useState<string | null>(null);

  // Virtual scroll container
  const containerRef = useRef<HTMLDivElement>(null);

  // Load data
  const loadGroups = useCallback(() => {
    setLoading(true);
    api
      .get<DataGroupRow[]>("/data/groups")
      .then((data) => {
        setGroups(data);
        setSelectedGroup((prev) =>
          prev ? data.find((g) => g.id === prev.id) ?? null : null,
        );
      })
      .catch(() => setGroups([]))
      .finally(() => setLoading(false));
  }, []);

  const [searchParams, setSearchParams] = useSearchParams();
  const initialGroupId = useRef(searchParams.get("group"));

  useEffect(() => {
    loadGroups();
  }, [loadGroups]);

  useEffect(() => {
    if (initialGroupId.current && groups && groups.length > 0) {
      const gid = Number(initialGroupId.current);
      initialGroupId.current = null;
      const found = groups.find((g) => g.id === gid);
      if (found) {
        setSelectedGroup(found);
        setDetailOpen(true);
      }
      setSearchParams({}, { replace: true });
    }
  }, [groups, setSearchParams]);

  // Apply kind filter as column filter
  useEffect(() => {
    if (kindFilter) {
      setColumnFilters([{ id: "training_kind", value: kindFilter }]);
    } else {
      setColumnFilters([]);
    }
  }, [kindFilter]);

  // Apply grouping
  useEffect(() => {
    setGrouping(groupBy ? [groupBy] : []);
  }, [groupBy]);

  // Kind stats for filter chips
  const kindStats = useMemo(() => {
    if (!groups) return [];
    const counts = new Map<string, number>();
    for (const g of groups) {
      const k = g.training_kind || "—";
      counts.set(k, (counts.get(k) ?? 0) + 1);
    }
    return Array.from(counts.entries()).sort((a, b) => b[1] - a[1]);
  }, [groups]);

  // Handlers
  const handleUpdate = useCallback(
    async (id: number, field: string, value: string) => {
      try {
        await api.put(`/data/groups/${id}`, { field, value });
        loadGroups();
      } catch {
        toast.error("Не вдалося оновити поле");
      }
    },
    [loadGroups],
  );

  const handleDelete = useCallback(async () => {
    if (!deleteTarget) return;
    setDeleting(true);
    try {
      await api.delete(`/data/groups/${deleteTarget.id}`);
      setDeleteTarget(null);
      loadGroups();
    } catch {
      toast.error("Не вдалося видалити групу");
    } finally {
      setDeleting(false);
    }
  }, [deleteTarget, loadGroups]);

  const handleDetail = useCallback((row: DataGroupRow) => {
    setSelectedGroup(row);
    setDetailOpen(true);
  }, []);

  const ctxMenu = useContextMenu();

  function handleRowContextMenu(e: React.MouseEvent, row: DataGroupRow) {
    const items: ContextMenuEntry[] = [
      {
        label: "Деталі",
        icon: <Eye className="h-4 w-4" />,
        onClick: () => handleDetail(row),
      },
      { separator: true },
      {
        label: "Копіювати підрозділ",
        icon: <Copy className="h-4 w-4" />,
        onClick: () => { navigator.clipboard.writeText(row.org_label); },
      },
      ...(isAdmin ? [
        { separator: true } as ContextMenuEntry,
        {
          label: "Видалити",
          icon: <Trash2 className="h-4 w-4" />,
          variant: "destructive" as const,
          onClick: () => setDeleteTarget(row),
        },
      ] : []),
    ];
    ctxMenu.open(e, items);
  }

  // Columns
  const columns = useMemo(
    () => buildColumns(handleUpdate, setDeleteTarget, handleDetail, isAdmin),
    [handleUpdate, handleDetail, isAdmin],
  );

  // Table instance
  const table = useReactTable({
    data: groups ?? [],
    columns,
    state: {
      sorting,
      globalFilter,
      columnFilters,
      grouping,
    },
    onSortingChange: setSorting,
    onGlobalFilterChange: setGlobalFilter,
    onColumnFiltersChange: setColumnFilters,
    onGroupingChange: setGrouping,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getGroupedRowModel: getGroupedRowModel(),
    getExpandedRowModel: getExpandedRowModel(),
    globalFilterFn: "includesString",
  });

  const { rows } = table.getRowModel();

  // Virtual scroll
  const rowVirtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => containerRef.current,
    estimateSize: () => 40,
    overscan: 20,
  });

  // Summary stats from the pre-grouping filtered row model
  const stats = useMemo(() => {
    const filtered = table.getFilteredRowModel().rows;
    let planned = 0;
    let arrived = 0;
    let training = 0;
    let completed = 0;
    for (const row of filtered) {
      const g = row.original;
      planned += g.planned_count;
      arrived += g.arrived_count;
      training += g.in_training_count;
      completed += g.completed_count;
    }
    return { groups: filtered.length, planned, arrived, training, completed };
  }, [table.getFilteredRowModel().rows]);

  return (
    <div className="flex h-full flex-col gap-4">
      {/* Header + Stats */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-bold">Облік</h1>
        {groups !== null && groups.length > 0 && (
          <div className="flex flex-wrap gap-5">
            <Stat label="груп" value={stats.groups} />
            <Stat label="план" value={stats.planned} />
            <Stat label="прибуло" value={stats.arrived} />
            <Stat label="навч." value={stats.training} />
            <Stat label="заверш." value={stats.completed} />
          </div>
        )}
      </div>

      {/* Toolbar */}
      <div className="flex flex-wrap items-center gap-2">
        {/* Search */}
        <div className="relative w-full max-w-xs">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Пошук за підрозділом, ВОС, полігоном..."
            value={globalFilter}
            onChange={(e) => setGlobalFilter(e.target.value)}
            className="pl-9"
          />
          {globalFilter && (
            <Button
              variant="ghost"
              size="icon"
              className="absolute right-2 top-1/2 h-6 w-6 -translate-y-1/2"
              onClick={() => setGlobalFilter("")}
            >
              <X className="h-3.5 w-3.5 text-muted-foreground" />
            </Button>
          )}
        </div>

        {/* Kind filter chips */}
        {kindStats.length > 1 && (
          <>
            <Button
              variant={!kindFilter ? "default" : "outline"}
              size="sm"
              onClick={() => setKindFilter(null)}
              className="text-xs font-semibold uppercase tracking-wider"
            >
              Усі ({groups?.length ?? 0})
            </Button>
            {kindStats.map(([kind, count]) => (
              <Button
                key={kind}
                variant={kindFilter === kind ? "default" : "outline"}
                size="sm"
                onClick={() =>
                  setKindFilter(kindFilter === kind ? null : kind)
                }
                className="text-xs font-semibold"
                style={{ letterSpacing: "0.02em" }}
              >
                {kind} ({count})
              </Button>
            ))}
          </>
        )}

        <div className="flex-1" />

        {/* Group by selector */}
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="outline"
              className="h-9 gap-2"
              style={{
                color: groupBy ? "var(--primary)" : "var(--muted-foreground)",
              }}
            >
              <Layers className="h-3.5 w-3.5" />
              {groupBy === "training_kind"
                ? "За видом"
                : groupBy === "org_label"
                  ? "За підрозділом"
                  : groupBy === "site_label"
                    ? "За місцем"
                    : "Групувати"}
              <ChevronDown className="h-3 w-3 opacity-50" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onClick={() => setGroupBy(null)}>
              Без групування
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={() => setGroupBy("training_kind")}>
              За видом підготовки
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => setGroupBy("org_label")}>
              За підрозділом
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => setGroupBy("site_label")}>
              За місцем проведення
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>

        {/* Create group (admin only) */}
        {isAdmin && (
          <Button size="sm" onClick={() => setShowCreateDialog(true)}>
            <Plus className="mr-1 h-4 w-4" />
            Нова група
          </Button>
        )}
      </div>

      {/* Import section (admin only) */}
      {isAdmin && <ImportSection onImported={loadGroups} />}

      {/* Data grid */}
      {loading ? (
        <div className="flex flex-1 items-center justify-center">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      ) : rows.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-3 py-16">
          <Users className="h-10 w-10 text-muted-foreground" />
          <p className="text-sm text-muted-foreground">
            {globalFilter || kindFilter
              ? "Нічого не знайдено за цим фільтром"
              : "Груп підготовки ще немає"}
          </p>
          {!globalFilter && !kindFilter && isAdmin && (
            <Button size="sm" onClick={() => setShowCreateDialog(true)}>
              <Plus className="mr-1 h-4 w-4" />
              Створити першу групу
            </Button>
          )}
        </div>
      ) : (
        <div
          ref={containerRef}
          className="flex-1 overflow-auto rounded-lg border"
          style={{ borderColor: "var(--border)", maxHeight: "calc(100vh - 260px)" }}
        >
          <table className="border-collapse text-sm" style={{ minWidth: "100%", width: "max-content" }}>
            <thead className="sticky top-0 z-10" style={{ background: "var(--background)" }}>
              {table.getHeaderGroups().map((hg) => (
                <tr key={hg.id}>
                  {hg.headers.map((header) => (
                    <th
                      key={header.id}
                      className="whitespace-nowrap border-b px-3 py-2 text-left text-[11px] font-semibold uppercase tracking-wider select-none"
                      style={{
                        width: header.getSize(),
                        color: "var(--muted-foreground)",
                        borderColor: "var(--border)",
                        cursor: header.column.getCanSort()
                          ? "pointer"
                          : "default",
                        textAlign:
                          (header.column.columnDef.meta as { align?: string })
                            ?.align ?? "left",
                      }}
                      onClick={header.column.getToggleSortingHandler()}
                    >
                      {header.isPlaceholder
                        ? null
                        : flexRender(
                            header.column.columnDef.header,
                            header.getContext(),
                          )}
                      {header.column.getCanSort() && (
                        <SortIcon sorted={header.column.getIsSorted()} />
                      )}
                    </th>
                  ))}
                </tr>
              ))}
            </thead>
            <tbody>
              {rowVirtualizer.getVirtualItems().map((virtualRow) => {
                const row = rows[virtualRow.index];
                if (!row) return null;

                if (row.getIsGrouped()) {
                  return (
                    <tr
                      key={row.id}
                      className="cursor-pointer hover:bg-accent/30"
                      onClick={row.getToggleExpandedHandler()}
                    >
                      <td
                        colSpan={columns.length}
                        className="border-b px-3 py-2 font-semibold"
                        style={{ borderColor: "var(--border)" }}
                      >
                        {row.getIsExpanded() ? (
                          <ChevronDown className="mr-1 inline h-4 w-4" />
                        ) : (
                          <ChevronRight className="mr-1 inline h-4 w-4" />
                        )}
                        {String(row.groupingValue)}{" "}
                        <span className="font-normal text-muted-foreground">
                          ({row.subRows.length})
                        </span>
                      </td>
                    </tr>
                  );
                }

                return (
                  <tr
                    key={row.id}
                    className="cursor-pointer hover:bg-accent/40"
                    style={{
                      height: virtualRow.size,
                      background:
                        virtualRow.index % 2 === 0
                          ? "transparent"
                          : "color-mix(in srgb, var(--muted) 20%, transparent)",
                    }}
                    onClick={() => handleDetail(row.original)}
                    onContextMenu={(e) => handleRowContextMenu(e, row.original)}
                  >
                    {row.getVisibleCells().map((cell) => (
                      <td
                        key={cell.id}
                        className="border-b px-3 py-1.5"
                        style={{
                          borderColor: "var(--border)",
                          textAlign:
                            (cell.column.columnDef.meta as { align?: string })
                              ?.align ?? "left",
                        }}
                        onClick={(e) => {
                          if (cell.column.id === "actions") e.stopPropagation();
                        }}
                      >
                        {flexRender(
                          cell.column.columnDef.cell,
                          cell.getContext(),
                        )}
                      </td>
                    ))}
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      {/* Detail panel */}
      <DetailPanel
        group={selectedGroup}
        open={detailOpen}
        onOpenChange={(v) => {
          setDetailOpen(v);
          if (!v) setSelectedGroup(null);
        }}
        onGroupChanged={loadGroups}
      />

      {/* Delete confirmation dialog */}
      <Dialog
        open={deleteTarget !== null}
        onOpenChange={(open) => {
          if (!open) setDeleteTarget(null);
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Видалити групу?</DialogTitle>
            <DialogDescription>
              {deleteTarget && (
                <>
                  Група #{deleteTarget.id} — {deleteTarget.org_label},{" "}
                  {deleteTarget.training_kind}. Усі пов'язані події також буде
                  видалено. Цю дію неможливо скасувати.
                </>
              )}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteTarget(null)}>
              Скасувати
            </Button>
            <Button
              variant="destructive"
              onClick={handleDelete}
              disabled={deleting}
            >
              {deleting ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <Trash2 className="mr-2 h-4 w-4" />
              )}
              Видалити
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Create group dialog */}
      <CreateGroupDialog
        open={showCreateDialog}
        onOpenChange={setShowCreateDialog}
        onCreated={loadGroups}
      />

      <ContextMenuPortal state={ctxMenu.state} onClose={ctxMenu.close} />
    </div>
  );
}
