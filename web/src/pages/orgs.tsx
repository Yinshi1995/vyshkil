import { useEffect, useMemo, useState, useCallback } from "react";
import { api } from "@/api/client";
import type {
  OrgHierarchyNode,
  OrgNumber,
  SubordinationLink,
  OrgSearchResult,
} from "@/api/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Separator } from "@/components/ui/separator";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Command,
  CommandEmpty,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { toast } from "sonner";
import { useAuth } from "@/context/auth";
import {
  Building2,
  ChevronRight,
  ChevronDown,
  Plus,
  Search,
  Trash2,
  ChevronsUpDown,
  Check,
  Shield,
  GitBranch,
  Hash,
} from "lucide-react";

const ORG_KINDS: { value: string; label: string }[] = [
  { value: "military_unit", label: "Військова частина" },
  { value: "command", label: "Командний орган" },
  { value: "virtual_group", label: "Віртуальна група" },
  { value: "subunit", label: "Підрозділ" },
  { value: "edu_institution", label: "Навчальний заклад" },
  { value: "company", label: "Компанія" },
  { value: "ngo", label: "НГО" },
  { value: "foreign_state", label: "Іноземна держава" },
  { value: "other", label: "Інше" },
];

function kindLabel(kind: string): string {
  return ORG_KINDS.find((k) => k.value === kind)?.label ?? kind;
}

// ---------------------------------------------------------------------------
// Org Combobox (reusable search-based org picker)
// ---------------------------------------------------------------------------

function OrgCombobox({
  value,
  label,
  onChange,
  exclude,
}: {
  value: number | null;
  label: string;
  onChange: (orgId: number, orgLabel: string) => void;
  exclude?: number;
}) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<OrgSearchResult[]>([]);

  useEffect(() => {
    const t = setTimeout(() => {
      api
        .get<OrgSearchResult[]>(
          `/orgs/search?q=${encodeURIComponent(query)}&scope=visible`
        )
        .then((r) =>
          setResults(exclude ? r.filter((o) => o.org_id !== exclude) : r)
        )
        .catch(() => {});
    }, 200);
    return () => clearTimeout(t);
  }, [query, exclude]);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger
        render={(props) => (
          <Button
            variant="outline"
            className="w-full justify-between font-normal"
            {...props}
          >
            <span className="truncate">
              {value ? label : "Оберіть підрозділ…"}
            </span>
            <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
          </Button>
        )}
      />
      <PopoverContent className="w-[--anchor-width] p-0" align="start">
        <Command shouldFilter={false}>
          <CommandInput
            placeholder="Пошук…"
            value={query}
            onValueChange={setQuery}
          />
          <CommandList>
            <CommandEmpty>Не знайдено</CommandEmpty>
            {results.map((r) => (
              <CommandItem
                key={r.org_id}
                onSelect={() => {
                  onChange(r.org_id, r.label);
                  setOpen(false);
                }}
              >
                <Check
                  className={`mr-2 h-4 w-4 ${value === r.org_id ? "opacity-100" : "opacity-0"}`}
                />
                {r.label}
              </CommandItem>
            ))}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}

// ---------------------------------------------------------------------------
// Tree Node (recursive)
// ---------------------------------------------------------------------------

interface TreeNodeData {
  node: OrgHierarchyNode;
  children: TreeNodeData[];
}

function buildTree(
  nodes: OrgHierarchyNode[]
): { roots: TreeNodeData[]; orphans: TreeNodeData[] } {
  const map = new Map<number, TreeNodeData>();
  for (const n of nodes) {
    map.set(n.id, { node: n, children: [] });
  }

  const roots: TreeNodeData[] = [];
  const orphans: TreeNodeData[] = [];

  for (const n of nodes) {
    const td = map.get(n.id)!;
    if (n.parent_id && map.has(n.parent_id)) {
      map.get(n.parent_id)!.children.push(td);
    } else if (n.parent_id === null) {
      roots.push(td);
    } else {
      orphans.push(td);
    }
  }

  const sortChildren = (list: TreeNodeData[]) => {
    list.sort((a, b) => a.node.short_name.localeCompare(b.node.short_name));
    for (const c of list) sortChildren(c.children);
  };
  sortChildren(roots);
  sortChildren(orphans);

  return { roots, orphans };
}

function filterTree(td: TreeNodeData, q: string): TreeNodeData | null {
  if (td.node.short_name.toLowerCase().includes(q)) return td;
  const fc = td.children
    .map((c) => filterTree(c, q))
    .filter(Boolean) as TreeNodeData[];
  if (fc.length > 0) return { ...td, children: fc };
  return null;
}

function TreeNode({
  data,
  depth,
  selectedId,
  onSelect,
  expanded,
  onToggle,
}: {
  data: TreeNodeData;
  depth: number;
  selectedId: number | null;
  onSelect: (id: number) => void;
  expanded: Set<number>;
  onToggle: (id: number) => void;
}) {
  const hasChildren = data.children.length > 0;
  const isExpanded = expanded.has(data.node.id);
  const isSelected = selectedId === data.node.id;

  return (
    <>
      <button
        className={`flex w-full items-center gap-1 rounded-md px-2 py-1 text-left text-sm hover:bg-muted/50 ${
          isSelected ? "bg-primary/10 font-medium text-primary" : ""
        } ${!data.node.is_active ? "opacity-50" : ""}`}
        style={{ paddingLeft: depth * 16 + 8 }}
        onClick={() => onSelect(data.node.id)}
      >
        {hasChildren ? (
          <button
            className="shrink-0 p-0.5 hover:bg-muted rounded"
            onClick={(e) => {
              e.stopPropagation();
              onToggle(data.node.id);
            }}
          >
            {isExpanded ? (
              <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" />
            ) : (
              <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
            )}
          </button>
        ) : (
          <span className="w-[22px] shrink-0" />
        )}
        <Building2 className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
        <span className="truncate">{data.node.short_name}</span>
      </button>
      {hasChildren && isExpanded && (
        <div>
          {data.children.map((c) => (
            <TreeNode
              key={c.node.id}
              data={c}
              depth={depth + 1}
              selectedId={selectedId}
              onSelect={onSelect}
              expanded={expanded}
              onToggle={onToggle}
            />
          ))}
        </div>
      )}
    </>
  );
}

// ---------------------------------------------------------------------------
// Detail Panel — Tab "Основне"
// ---------------------------------------------------------------------------

function BasicTab({
  org,
  onSaved,
}: {
  org: OrgHierarchyNode;
  onSaved: () => void;
}) {
  const [name, setName] = useState(org.short_name);
  const [kind, setKind] = useState(org.kind);
  const [echelon, setEchelon] = useState(org.echelon ?? "");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setName(org.short_name);
    setKind(org.kind);
    setEchelon(org.echelon ?? "");
  }, [org.id, org.short_name, org.kind, org.echelon]);

  async function handleSave() {
    setSaving(true);
    try {
      await api.put(`/admin/orgs/${org.id}`, {
        short_name: name,
        kind,
        echelon: echelon || null,
      });
      toast.success("Збережено");
      onSaved();
    } catch {
      toast.error("Помилка збереження");
    } finally {
      setSaving(false);
    }
  }

  async function handleDeactivate() {
    if (!confirm("Деактивувати підрозділ? Його можна буде відновити.")) return;
    try {
      await api.delete(`/admin/orgs/${org.id}`);
      toast.success("Деактивовано");
      onSaved();
    } catch {
      toast.error("Помилка — можливо, є пов'язані дані");
    }
  }

  return (
    <div className="flex flex-col gap-5">
      <div className="flex items-center gap-3">
        <h2 className="text-lg font-semibold">{org.short_name}</h2>
        <Badge variant={org.is_active ? "default" : "secondary"}>
          {org.is_active ? "Активний" : "Неактивний"}
        </Badge>
      </div>
      <Separator />

      <div className="grid gap-4 sm:grid-cols-2">
        <div className="flex flex-col gap-1.5">
          <Label>Назва (скорочена)</Label>
          <Input value={name} onChange={(e) => setName(e.target.value)} />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label>Тип</Label>
          <Select value={kind} onValueChange={setKind}>
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {ORG_KINDS.map((k) => (
                <SelectItem key={k.value} value={k.value}>
                  {k.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <div className="flex flex-col gap-1.5">
          <Label>Ешелон (необов'язково)</Label>
          <Input
            value={echelon}
            onChange={(e) => setEchelon(e.target.value)}
            placeholder="напр. ак, омбр"
          />
        </div>
      </div>

      <div className="flex gap-2">
        <Button onClick={handleSave} disabled={saving || !name.trim()}>
          {saving ? "Збереження…" : "Зберегти"}
        </Button>
        {org.is_active && (
          <Button variant="destructive" onClick={handleDeactivate}>
            <Trash2 className="mr-1.5 h-3.5 w-3.5" />
            Деактивувати
          </Button>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Detail Panel — Tab "Код ВЧ" (ДСК: NO org name visible)
// ---------------------------------------------------------------------------

function NumberTab({ orgId }: { orgId: number }) {
  const [data, setData] = useState<OrgNumber | null>(null);
  const [numberKind, setNumberKind] = useState("");
  const [number, setNumber] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setData(null);
    api
      .get<OrgNumber>(`/admin/orgs/${orgId}/number`)
      .then((d) => {
        setData(d);
        setNumberKind(d.number_kind ?? "");
        setNumber(d.number ?? "");
      })
      .catch(() => {});
  }, [orgId]);

  async function handleSave() {
    setSaving(true);
    try {
      await api.put(`/admin/orgs/${orgId}/number`, {
        number_kind: numberKind || null,
        number: number || null,
      });
      toast.success("Код збережено");
    } catch {
      toast.error("Помилка збереження");
    } finally {
      setSaving(false);
    }
  }

  if (!data) return <Skeleton className="h-32 w-full" />;

  return (
    <div className="flex flex-col gap-5">
      <div className="flex items-center gap-2">
        <Hash className="h-5 w-5 text-primary" />
        <h2 className="text-lg font-semibold">Код ВЧ</h2>
      </div>
      <p className="text-xs text-muted-foreground">
        Номер військової частини. З міркувань безпеки (ДСК) назва підрозділу тут
        не відображається.
      </p>
      <Separator />

      <div className="grid gap-4 sm:grid-cols-2 max-w-md">
        <div className="flex flex-col gap-1.5">
          <Label>Літера</Label>
          <Select
            value={numberKind || "__none__"}
            onValueChange={(v) => setNumberKind(v === "__none__" ? "" : v)}
          >
            <SelectTrigger>
              <SelectValue placeholder="—" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="__none__">—</SelectItem>
              <SelectItem value="A">А</SelectItem>
              <SelectItem value="T">Т</SelectItem>
              <SelectItem value="NGU">НГУ</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div className="flex flex-col gap-1.5">
          <Label>Номер</Label>
          <Input
            value={number}
            onChange={(e) => setNumber(e.target.value)}
            placeholder="напр. 7384"
          />
        </div>
      </div>

      <Button className="self-start" onClick={handleSave} disabled={saving}>
        {saving ? "Збереження…" : "Зберегти"}
      </Button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Detail Panel — Tab "Підпорядкування"
// ---------------------------------------------------------------------------

function SubordinationTab({
  orgId,
  orgName,
  onSelectOrg,
}: {
  orgId: number;
  orgName: string;
  onSelectOrg: (id: number) => void;
}) {
  const [parents, setParents] = useState<SubordinationLink[]>([]);
  const [children, setChildren] = useState<SubordinationLink[]>([]);
  const [loading, setLoading] = useState(true);
  const [addOpen, setAddOpen] = useState(false);
  const [addParentOrgId, setAddParentOrgId] = useState<number | null>(null);
  const [addParentLabel, setAddParentLabel] = useState("");
  const [addAxis, setAddAxis] = useState("staff");
  const [addFrom, setAddFrom] = useState("");

  const load = useCallback(() => {
    setLoading(true);
    api
      .get<{ parents: SubordinationLink[]; children: SubordinationLink[] }>(
        `/admin/orgs/${orgId}/subordination`
      )
      .then((d) => {
        setParents(d.parents);
        setChildren(d.children);
      })
      .catch(() => {})
      .finally(() => setLoading(false));
  }, [orgId]);

  useEffect(() => {
    load();
  }, [load]);

  async function handleClose(subId: number) {
    const validTo = prompt("Дата закриття (YYYY-MM-DD):");
    if (!validTo) return;
    try {
      await api.put(`/admin/subordination/${subId}/close`, {
        valid_to: validTo,
      });
      toast.success("Закрито");
      load();
    } catch {
      toast.error("Помилка");
    }
  }

  async function handleDeleteSub(subId: number) {
    if (!confirm("Видалити зв'язок підпорядкування?")) return;
    try {
      await api.delete(`/admin/subordination/${subId}`);
      toast.success("Видалено");
      load();
    } catch {
      toast.error("Помилка видалення");
    }
  }

  async function handleAddSub() {
    if (!addParentOrgId || !addFrom) return;
    try {
      await api.post("/admin/subordination", {
        child_org_id: orgId,
        parent_org_id: addParentOrgId,
        axis: addAxis,
        valid_from: addFrom,
      });
      toast.success("Додано");
      setAddOpen(false);
      setAddParentOrgId(null);
      setAddParentLabel("");
      setAddFrom("");
      load();
    } catch {
      toast.error("Помилка — можливо, період перетинається з наявним");
    }
  }

  if (loading) return <Skeleton className="h-48 w-full" />;

  return (
    <div className="flex flex-col gap-5">
      <div className="flex items-center gap-2">
        <GitBranch className="h-5 w-5 text-primary" />
        <h2 className="text-lg font-semibold">Підпорядкування</h2>
        <span className="text-sm text-muted-foreground">{orgName}</span>
      </div>
      <Separator />

      {/* Parents */}
      <div className="flex flex-col gap-2">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-medium">Підпорядковується</h3>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setAddOpen(true)}
          >
            <Plus className="mr-1.5 h-3.5 w-3.5" />
            Додати
          </Button>
        </div>
        {parents.length === 0 ? (
          <p className="text-sm text-muted-foreground py-2">
            Немає зв'язків підпорядкування
          </p>
        ) : (
          <div className="rounded-lg border border-border overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="bg-muted/50 text-muted-foreground">
                  <th className="px-3 py-2 text-left font-medium">
                    Керівник
                  </th>
                  <th className="px-3 py-2 text-left font-medium">Вісь</th>
                  <th className="px-3 py-2 text-left font-medium">З</th>
                  <th className="px-3 py-2 text-left font-medium">По</th>
                  <th className="px-3 py-2 text-right font-medium">Дії</th>
                </tr>
              </thead>
              <tbody>
                {parents.map((p) => (
                  <tr key={p.id} className="border-t border-border">
                    <td className="px-3 py-2">
                      <button
                        className="text-primary hover:underline"
                        onClick={() => onSelectOrg(p.other_org_id)}
                      >
                        {p.other_org_name}
                      </button>
                    </td>
                    <td className="px-3 py-2">
                      <Badge
                        variant={
                          p.axis === "staff" ? "default" : "secondary"
                        }
                        className="text-xs"
                      >
                        {p.axis === "staff" ? "Штатне" : "Оперативне"}
                      </Badge>
                    </td>
                    <td className="px-3 py-2 tabular-nums">{p.valid_from}</td>
                    <td className="px-3 py-2 tabular-nums text-muted-foreground">
                      {p.valid_to ?? "—"}
                    </td>
                    <td className="px-3 py-2 text-right">
                      <div className="flex justify-end gap-1">
                        {!p.valid_to && (
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => handleClose(p.id)}
                          >
                            Закрити
                          </Button>
                        )}
                        <Button
                          variant="ghost"
                          size="icon"
                          onClick={() => handleDeleteSub(p.id)}
                        >
                          <Trash2 className="h-3.5 w-3.5 text-destructive" />
                        </Button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <Separator />

      {/* Children */}
      <div className="flex flex-col gap-2">
        <h3 className="text-sm font-medium">
          Підлеглі{" "}
          <span className="text-muted-foreground font-normal">
            ({children.length})
          </span>
        </h3>
        {children.length === 0 ? (
          <p className="text-sm text-muted-foreground py-2">
            Немає підлеглих підрозділів
          </p>
        ) : (
          <div className="flex flex-wrap gap-1.5">
            {children.map((c) => (
              <button
                key={c.id}
                className="inline-flex items-center gap-1 rounded-md border border-border px-2 py-1 text-sm hover:bg-muted/50"
                onClick={() => onSelectOrg(c.other_org_id)}
              >
                <Building2 className="h-3 w-3 text-muted-foreground" />
                {c.other_org_name}
                {c.axis !== "staff" && (
                  <Badge variant="secondary" className="text-[10px] px-1 h-4">
                    опер
                  </Badge>
                )}
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Add Subordination Dialog */}
      <Dialog open={addOpen} onOpenChange={setAddOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Додати підпорядкування</DialogTitle>
          </DialogHeader>
          <p className="text-sm text-muted-foreground">
            {orgName} буде підпорядкований обраному підрозділу.
          </p>
          <div className="flex flex-col gap-4 pt-2">
            <div className="flex flex-col gap-1.5">
              <Label>Керівник</Label>
              <OrgCombobox
                value={addParentOrgId}
                label={addParentLabel}
                onChange={(id, label) => {
                  setAddParentOrgId(id);
                  setAddParentLabel(label);
                }}
                exclude={orgId}
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label>Вісь підпорядкування</Label>
              <Select value={addAxis} onValueChange={setAddAxis}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="staff">Штатне</SelectItem>
                  <SelectItem value="operational">Оперативне</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="flex flex-col gap-1.5">
              <Label>Дата з</Label>
              <Input
                type="date"
                value={addFrom}
                onChange={(e) => setAddFrom(e.target.value)}
              />
            </div>
            <Button
              onClick={handleAddSub}
              disabled={!addParentOrgId || !addFrom}
            >
              Додати
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Create Org Dialog
// ---------------------------------------------------------------------------

function CreateOrgDialog({
  open,
  onOpenChange,
  onCreated,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreated: () => void;
}) {
  const [name, setName] = useState("");
  const [kind, setKind] = useState("military_unit");
  const [echelon, setEchelon] = useState("");
  const [parentOrgId, setParentOrgId] = useState<number | null>(null);
  const [parentLabel, setParentLabel] = useState("");
  const [saving, setSaving] = useState(false);

  async function handleCreate() {
    if (!name.trim()) return;
    setSaving(true);
    try {
      const res = await api.post<{ id: number }>("/admin/orgs", {
        short_name: name.trim(),
        kind,
        echelon: echelon || null,
      });

      if (parentOrgId) {
        const today = new Date().toISOString().slice(0, 10);
        await api.post("/admin/subordination", {
          child_org_id: res.id,
          parent_org_id: parentOrgId,
          axis: "staff",
          valid_from: today,
        });
      }

      toast.success("Підрозділ створено");
      onOpenChange(false);
      setName("");
      setKind("military_unit");
      setEchelon("");
      setParentOrgId(null);
      setParentLabel("");
      onCreated();
    } catch {
      toast.error("Помилка створення");
    } finally {
      setSaving(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Новий підрозділ</DialogTitle>
        </DialogHeader>
        <div className="flex flex-col gap-4 pt-2">
          <div className="flex flex-col gap-1.5">
            <Label>Назва (скорочена)</Label>
            <Input
              placeholder="напр. 87 омбр"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label>Тип</Label>
            <Select value={kind} onValueChange={setKind}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {ORG_KINDS.map((k) => (
                  <SelectItem key={k.value} value={k.value}>
                    {k.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="flex flex-col gap-1.5">
            <Label>Ешелон (необов'язково)</Label>
            <Input
              value={echelon}
              onChange={(e) => setEchelon(e.target.value)}
              placeholder="напр. ак, омбр"
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label>Керівник (необов'язково)</Label>
            <OrgCombobox
              value={parentOrgId}
              label={parentLabel}
              onChange={(id, label) => {
                setParentOrgId(id);
                setParentLabel(label);
              }}
            />
          </div>
          <Button onClick={handleCreate} disabled={saving || !name.trim()}>
            {saving ? "Створення…" : "Створити"}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

// ---------------------------------------------------------------------------
// Main Page
// ---------------------------------------------------------------------------

export function OrgsPage() {
  const { isAdmin } = useAuth();
  const [nodes, setNodes] = useState<OrgHierarchyNode[] | null>(null);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  const [search, setSearch] = useState("");
  const [createOpen, setCreateOpen] = useState(false);
  const [activeTab, setActiveTab] = useState("basic");

  const loadTree = useCallback(() => {
    if (!isAdmin) return;
    api
      .get<OrgHierarchyNode[]>("/admin/orgs/tree")
      .then(setNodes)
      .catch(() => setNodes([]));
  }, [isAdmin]);

  useEffect(() => {
    loadTree();
  }, [loadTree]);

  const { roots, orphans } = useMemo(() => {
    if (!nodes) return { roots: [], orphans: [] };
    return buildTree(nodes);
  }, [nodes]);

  const filteredRoots = useMemo(() => {
    if (!search) return roots;
    const q = search.toLowerCase();
    return roots
      .map((r) => filterTree(r, q))
      .filter(Boolean) as TreeNodeData[];
  }, [roots, search]);

  const filteredOrphans = useMemo(() => {
    if (!search) return orphans;
    const q = search.toLowerCase();
    return orphans.filter((o) =>
      o.node.short_name.toLowerCase().includes(q)
    );
  }, [orphans, search]);

  const selectedNode = useMemo(
    () => nodes?.find((n) => n.id === selectedId) ?? null,
    [nodes, selectedId]
  );

  function handleToggle(id: number) {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  function handleSelect(id: number) {
    setSelectedId(id);
    setActiveTab("basic");
    // Auto-expand ancestors
    if (nodes) {
      const node = nodes.find((n) => n.id === id);
      if (node?.parent_id) {
        setExpanded((prev) => {
          const next = new Set(prev);
          let pid: number | null = node.parent_id;
          while (pid) {
            next.add(pid);
            const parent = nodes.find((n) => n.id === pid);
            pid = parent?.parent_id ?? null;
          }
          return next;
        });
      }
    }
  }

  function handleRefresh() {
    loadTree();
  }

  if (!isAdmin) {
    return (
      <div className="flex flex-col items-center justify-center py-20">
        <Shield className="mb-3 h-12 w-12 text-muted-foreground/50" />
        <p className="text-muted-foreground">
          Ця сторінка доступна тільки адміністраторам
        </p>
      </div>
    );
  }

  if (nodes === null) {
    return (
      <div className="flex flex-col gap-4 p-6">
        <Skeleton className="h-8 w-48" />
        <div className="flex gap-4">
          <Skeleton className="h-[600px] w-80" />
          <Skeleton className="h-[600px] flex-1" />
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-border">
        <div className="flex items-center gap-3">
          <h1 className="text-2xl font-bold">Підрозділи</h1>
          <Badge variant="secondary">{nodes.length}</Badge>
        </div>
        <Button size="sm" onClick={() => setCreateOpen(true)}>
          <Plus className="mr-1.5 h-3.5 w-3.5" />
          Додати
        </Button>
      </div>

      {/* Body: Tree + Detail */}
      <div className="flex flex-1 min-h-0">
        {/* Tree Panel */}
        <div className="w-80 shrink-0 border-r border-border overflow-y-auto flex flex-col">
          {/* Search */}
          <div className="p-3 border-b border-border">
            <div className="relative">
              <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
              <Input
                placeholder="Пошук…"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="pl-8 h-8 text-sm"
              />
            </div>
          </div>

          {/* Tree */}
          <div className="flex-1 overflow-y-auto p-2">
            {filteredRoots.map((r) => (
              <TreeNode
                key={r.node.id}
                data={r}
                depth={0}
                selectedId={selectedId}
                onSelect={handleSelect}
                expanded={expanded}
                onToggle={handleToggle}
              />
            ))}

            {filteredOrphans.length > 0 && (
              <>
                <Separator className="my-2" />
                <p className="px-2 py-1 text-xs text-muted-foreground font-medium">
                  Без підпорядкування
                </p>
                {filteredOrphans.map((o) => (
                  <TreeNode
                    key={o.node.id}
                    data={o}
                    depth={0}
                    selectedId={selectedId}
                    onSelect={handleSelect}
                    expanded={expanded}
                    onToggle={handleToggle}
                  />
                ))}
              </>
            )}
          </div>
        </div>

        {/* Detail Panel */}
        <div className="flex-1 overflow-y-auto p-6">
          {selectedNode ? (
            <Tabs value={activeTab} onValueChange={setActiveTab}>
              <TabsList variant="line">
                <TabsTrigger value="basic">
                  <Building2 className="mr-1.5 h-3.5 w-3.5" />
                  Основне
                </TabsTrigger>
                <TabsTrigger value="number">
                  <Hash className="mr-1.5 h-3.5 w-3.5" />
                  Код ВЧ
                </TabsTrigger>
                <TabsTrigger value="subordination">
                  <GitBranch className="mr-1.5 h-3.5 w-3.5" />
                  Підпорядкування
                </TabsTrigger>
              </TabsList>

              <TabsContent value="basic" className="pt-4">
                <BasicTab org={selectedNode} onSaved={handleRefresh} />
              </TabsContent>

              <TabsContent value="number" className="pt-4">
                <NumberTab orgId={selectedNode.id} />
              </TabsContent>

              <TabsContent value="subordination" className="pt-4">
                <SubordinationTab
                  orgId={selectedNode.id}
                  orgName={selectedNode.short_name}
                  onSelectOrg={handleSelect}
                />
              </TabsContent>
            </Tabs>
          ) : (
            <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
              <Building2 className="mb-3 h-12 w-12 opacity-30" />
              <p>Оберіть підрозділ зліва</p>
            </div>
          )}
        </div>
      </div>

      {/* Create Dialog */}
      <CreateOrgDialog
        open={createOpen}
        onOpenChange={setCreateOpen}
        onCreated={handleRefresh}
      />
    </div>
  );
}
