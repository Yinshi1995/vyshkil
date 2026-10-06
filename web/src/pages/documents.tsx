import { useState, useCallback, useRef, useEffect } from "react";
import { api, downloadBlob } from "@/api/client";
import type { OrgSearchResult } from "@/api/types";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { DatePickerUa } from "@/components/ui/date-picker-ua";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import {
  FileSpreadsheet,
  FileText,
  Presentation,
  Download,
  Loader2,
  ChevronDown,
  ChevronRight,
} from "lucide-react";
import { buildD4Pptx, type D4Data } from "@/lib/d4-pptx";

// ---------------------------------------------------------------------------
// Org autocomplete
// ---------------------------------------------------------------------------

function OrgPicker({
  orgId,
  orgLabel,
  onSelect,
}: {
  orgId: number | null;
  orgLabel: string;
  onSelect: (id: number | null, label: string) => void;
}) {
  const [query, setQuery] = useState(orgLabel);
  const [results, setResults] = useState<OrgSearchResult[]>([]);
  const [open, setOpen] = useState(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout>>(undefined);
  const wrapperRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    setQuery(orgLabel);
  }, [orgLabel]);

  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (wrapperRef.current && !wrapperRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, []);

  const doSearch = useCallback((q: string) => {
    clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      api
        .get<OrgSearchResult[]>(`/orgs/search?q=${encodeURIComponent(q)}&limit=5&scope=visible`)
        .then((r) => {
          setResults(r);
          setOpen(r.length > 0);
        })
        .catch(() => setResults([]));
    }, 200);
  }, []);

  return (
    <div ref={wrapperRef} className="relative">
      <Input
        value={query}
        placeholder="Почніть вводити назву частини…"
        onChange={(e) => {
          const v = e.target.value;
          setQuery(v);
          onSelect(null, v);
          doSearch(v);
        }}
        onFocus={() => {
          if (results.length > 0) setOpen(true);
          else doSearch(query);
        }}
      />
      {open && results.length > 0 && (
        <div
          className="absolute left-0 right-0 top-full z-50 mt-1 max-h-48 overflow-auto rounded-md border border-border bg-popover shadow-lg"
        >
          {results.map((r) => (
            <button
              key={r.org_id}
              type="button"
              className="flex w-full items-center gap-2 px-3 py-2 text-left text-sm hover:bg-accent"
              onClick={() => {
                onSelect(r.org_id, r.label);
                setQuery(r.label);
                setOpen(false);
              }}
            >
              <span className="truncate">{r.label}</span>
              {!r.is_exact && (
                <span className="shrink-0 text-xs text-muted-foreground">
                  ({r.matched_raw})
                </span>
              )}
            </button>
          ))}
        </div>
      )}
      {orgId && (
        <Badge variant="outline" className="absolute right-2 top-1/2 -translate-y-1/2 text-xs">
          #{orgId}
        </Badge>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Document block types
// ---------------------------------------------------------------------------

const EXT_STYLES: Record<string, { color: string; bg: string; border: string }> = {
  xlsx: { color: "#33b066", bg: "rgba(33, 115, 70, 0.15)", border: "rgba(33, 115, 70, 0.5)" },
  pptx: { color: "#d4513d", bg: "rgba(183, 71, 42, 0.15)", border: "rgba(183, 71, 42, 0.5)" },
  docx: { color: "#4a8ad4", bg: "rgba(43, 87, 154, 0.15)", border: "rgba(43, 87, 154, 0.5)" },
};

function ExtBadge({ ext, className }: { ext: string; className?: string }) {
  const s = EXT_STYLES[ext];
  if (!s) return <Badge variant="outline" className={className}>{ext}</Badge>;
  return (
    <span
      className={`inline-flex items-center rounded-md px-2 py-0.5 text-xs font-semibold uppercase tracking-wider ${className ?? ""}`}
      style={{
        color: s.color,
        background: s.bg,
        border: `1px solid ${s.border}`,
        letterSpacing: "0.06em",
      }}
    >
      .{ext}
    </span>
  );
}

interface DocBlockProps {
  title: string;
  description: string;
  icon: React.ElementType;
  iconLabel: string;
  needsOrg: boolean;
  apiPath: string;
  filePrefix: string;
}

function DocBlock({
  title,
  description,
  icon: Icon,
  iconLabel,
  needsOrg,
  apiPath,
  filePrefix,
}: DocBlockProps) {
  const [orgId, setOrgId] = useState<number | null>(null);
  const [orgLabel, setOrgLabel] = useState("");
  const [date, setDate] = useState("");
  const [status, setStatus] = useState<string | null>(null);
  const [generating, setGenerating] = useState(false);

  async function handleGenerate() {
    if (needsOrg && !orgId) {
      setStatus("Оберіть частину");
      return;
    }
    if (!date) {
      setStatus("Оберіть дату");
      return;
    }
    setGenerating(true);
    setStatus("Генерую…");
    try {
      const body = needsOrg ? { org_id: orgId, date } : { date };
      const ext = apiPath.includes("d3") ? "docx" : apiPath.includes("d4") ? "pptx" : "xlsx";
      const label = needsOrg && orgLabel ? `_${orgLabel}` : "";
      const fallback = `${filePrefix}${label}_${date}.${ext}`;
      await downloadBlob(`/documents/${apiPath}`, body, fallback);
      setStatus("Готово ✓");
    } catch (e) {
      setStatus(`Помилка: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setGenerating(false);
    }
  }

  return (
    <Card className={`card-animate card-hover ${needsOrg ? "overflow-visible" : ""}`}>
      <CardHeader className="pb-3">
        <CardTitle className="flex items-center gap-2 text-base">
          <div className="icon-box" style={{ width: 32, height: 32 }}>
            <Icon className="h-4 w-4" />
          </div>
          {title}
          <ExtBadge ext={iconLabel} className="ml-auto" />
        </CardTitle>
      </CardHeader>
      <CardContent className={needsOrg ? "flex flex-col gap-3 overflow-visible" : "flex flex-col gap-3"}>
        <p className="text-sm text-muted-foreground">{description}</p>
        <div className="flex flex-wrap items-end gap-3 overflow-visible">
          {needsOrg && (
            <div className="relative flex min-w-[200px] flex-1 flex-col gap-1.5" style={{ overflow: "visible" }}>
              <Label className="text-xs">Частина</Label>
              <OrgPicker
                orgId={orgId}
                orgLabel={orgLabel}
                onSelect={(id, label) => {
                  setOrgId(id);
                  setOrgLabel(label);
                }}
              />
            </div>
          )}
          <div className="flex flex-col gap-1.5">
            <Label className="text-xs">Дата</Label>
            <DatePickerUa
              value={date}
              onChange={setDate}
              className="w-[180px]"
            />
          </div>
          <Button
            onClick={handleGenerate}
            disabled={generating}
            className="gap-1.5"
          >
            {generating ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Download className="h-4 w-4" />
            )}
            Згенерувати
          </Button>
          {status && (
            <span
              className="text-sm"
              style={{
                color: status.startsWith("Помилка")
                  ? "var(--destructive)"
                  : status === "Готово ✓"
                    ? "#6bbd6b"
                    : "#8a8577",
              }}
            >
              {status}
            </span>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// D4 — client-side pptx via pptxgenjs
// ---------------------------------------------------------------------------

function D4Block() {
  const [date, setDate] = useState("");
  const [status, setStatus] = useState<string | null>(null);
  const [generating, setGenerating] = useState(false);

  async function handleGenerate() {
    if (!date) { setStatus("Оберіть дату"); return; }
    setGenerating(true);
    setStatus("Генерую…");
    try {
      const res = await fetch("/api/documents/d4/data", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ date }),
        credentials: "same-origin",
      });
      if (!res.ok) {
        const text = await res.text().catch(() => "");
        let msg = `HTTP ${res.status}`;
        try { msg = JSON.parse(text).error || msg; } catch { if (text) msg = text; }
        throw new Error(msg);
      }
      const data: D4Data = await res.json();
      await buildD4Pptx(data);
      setStatus("Готово ✓");
    } catch (e) {
      setStatus(`Помилка: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setGenerating(false);
    }
  }

  return (
    <Card className="card-animate card-hover">
      <CardHeader className="pb-3">
        <CardTitle className="flex items-center gap-2 text-base">
          <div className="icon-box" style={{ width: 32, height: 32 }}>
            <Presentation className="h-4 w-4" />
          </div>
          D4 — Презентація підготовки
          <ExtBadge ext="pptx" className="ml-auto" />
        </CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <p className="text-sm text-muted-foreground">
          Зведена презентація стану підготовки по корпусах та підрозділах.
        </p>
        <div className="flex flex-wrap items-end gap-3">
          <div className="flex flex-col gap-1.5">
            <Label className="text-xs">Дата</Label>
            <DatePickerUa
              value={date}
              onChange={setDate}
              className="w-[180px]"
            />
          </div>
          <Button onClick={handleGenerate} disabled={generating} className="gap-1.5">
            {generating ? <Loader2 className="h-4 w-4 animate-spin" /> : <Download className="h-4 w-4" />}
            Згенерувати
          </Button>
          {status && (
            <span
              className="text-sm"
              style={{
                color: status.startsWith("Помилка") ? "var(--destructive)"
                  : status === "Готово ✓" ? "#6bbd6b" : "#8a8577",
              }}
            >
              {status}
            </span>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

const DOCS_BEFORE_D4: DocBlockProps[] = [
  {
    title: "D1 — Зведена таблиця підрозділу",
    description:
      "Тижневий звіт підрозділу: 7 денних аркушів + тижневий підсумок. Оберіть будь-який день потрібного тижня.",
    icon: FileSpreadsheet,
    iconLabel: "xlsx",
    needsOrg: true,
    apiPath: "d1",
    filePrefix: "D1_Зведена",
  },
  {
    title: "D2 — Контрольна зведена таблиця",
    description:
      "Накопичувальний тижневий звіт по всіх корпусах. Оберіть будь-який день тижня.",
    icon: FileSpreadsheet,
    iconLabel: "xlsx",
    needsOrg: false,
    apiPath: "d2",
    filePrefix: "D2_Контрольна",
  },
  {
    title: "D3 — Текст доповіді за добу",
    description:
      "Доповідна записка за один день: дані по корпусах зі змінами за добу.",
    icon: FileText,
    iconLabel: "docx",
    needsOrg: false,
    apiPath: "d3",
    filePrefix: "D3_Доповідь",
  },
];

const DOCS_AFTER_D5: DocBlockProps[] = [
  {
    title: "D6 — Звіт по переданих частинах",
    description:
      "Частини зі статусом \"передано\" — кому, в яких заходах підготовки.",
    icon: FileSpreadsheet,
    iconLabel: "xlsx",
    needsOrg: false,
    apiPath: "d6",
    filePrefix: "D6_Передані",
  },
];

const D5_DOCS: DocBlockProps[] = [
  {
    title: "Фахова підготовка",
    description: "Аркуші \"Пройшли\" / \"Проходять\" з деталями по кожній групі.",
    icon: FileSpreadsheet,
    iconLabel: "xlsx",
    needsOrg: true,
    apiPath: "d5/fah",
    filePrefix: "D5_Фах",
  },
  {
    title: "БпС — безпілотні системи",
    description: '"Завершилась" / "Навчаються" по корпусу.',
    icon: FileSpreadsheet,
    iconLabel: "xlsx",
    needsOrg: true,
    apiPath: "d5/bps",
    filePrefix: "D5_БпС",
  },
  {
    title: "КВід — командири відділень",
    description: "Укомплектованість командирами відділень.",
    icon: FileSpreadsheet,
    iconLabel: "xlsx",
    needsOrg: true,
    apiPath: "d5/kvid",
    filePrefix: "D5_КВід",
  },
  {
    title: "ІВС — інструктори",
    description: "Укомплектованість інструкторів.",
    icon: FileSpreadsheet,
    iconLabel: "xlsx",
    needsOrg: true,
    apiPath: "d5/ivs",
    filePrefix: "D5_ІВС",
  },
  {
    title: "Терміни підготовки",
    description: "БЗВП / Фахова / Адаптація — кількість, терміни, місце.",
    icon: FileSpreadsheet,
    iconLabel: "xlsx",
    needsOrg: true,
    apiPath: "d5/terminy",
    filePrefix: "D5_Терміни",
  },
];

export function DocumentsPage() {
  const [d5Open, setD5Open] = useState(false);

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1>Документи</h1>
        <p className="text-sm text-muted-foreground">
          Генерація звітних документів за даними системи. Оберіть тип документа,
          вкажіть параметри і натисніть "Згенерувати".
        </p>
      </div>

      <Separator />

      <div className="eyebrow">Зведені таблиці та доповіді</div>

      <div className="grid gap-4 lg:grid-cols-2">
        {DOCS_BEFORE_D4.map((doc) => (
          <DocBlock key={doc.apiPath} {...doc} />
        ))}
        <D4Block />
      </div>

      <Separator />

      <div className="eyebrow">Додатки корпусу (D5)</div>

      <Card className="card-animate card-hover">
        <CardHeader className="pb-3">
          <Button
            variant="ghost"
            className="flex w-full justify-start gap-2 p-0 h-auto"
            onClick={() => setD5Open(!d5Open)}
          >
            <div className="icon-box" style={{ width: 32, height: 32 }}>
              <FileSpreadsheet className="h-4 w-4" />
            </div>
            <CardTitle className="flex-1 text-base">
              Деталізація по видах підготовки
            </CardTitle>
            <Badge variant="outline" className="text-xs font-normal">
              {D5_DOCS.length} файлів
            </Badge>
            {d5Open ? (
              <ChevronDown className="h-4 w-4 text-muted-foreground" />
            ) : (
              <ChevronRight className="h-4 w-4 text-muted-foreground" />
            )}
          </Button>
        </CardHeader>
        {d5Open && (
          <CardContent className="flex flex-col gap-4 pt-0">
            <div className="grid gap-4 lg:grid-cols-2">
              {D5_DOCS.map((doc) => (
                <DocBlock key={doc.apiPath} {...doc} />
              ))}
            </div>
          </CardContent>
        )}
      </Card>

      <Separator />

      <div className="eyebrow">Інші звіти</div>

      <div className="grid gap-4 lg:grid-cols-2">
        {DOCS_AFTER_D5.map((doc) => (
          <DocBlock key={doc.apiPath} {...doc} />
        ))}
      </div>
    </div>
  );
}
