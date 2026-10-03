import { useEffect, useRef, useState, type DragEvent, type ChangeEvent } from "react";
import { api } from "@/api/client";
import type { AdminSubmissionRow } from "@/api/types";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { toast } from "sonner";
import { Upload, FileUp, Clock, Download, Loader2 } from "lucide-react";
import { sourceTypeLabel, statusLabel, statusVariant } from "@/lib/labels";

type FileKind = "fah" | "bps" | "kvid" | "terminy" | "ivs" | "archive";

const FILE_KINDS: { value: FileKind; label: string; desc: string }[] = [
  { value: "fah", label: "Фах", desc: "Фахова підготовка (зведена таблиця)" },
  { value: "bps", label: "БпС", desc: "Бойова підготовка складових (зведена таблиця)" },
  { value: "kvid", label: "КВід", desc: "Укомплектованість командирами відділень" },
  { value: "terminy", label: "Терміни", desc: "Терміни підготовки (БЗВП/фахова/адаптація)" },
  { value: "ivs", label: "ІВС", desc: "Укомплектованість інструкторів + стажування/курси" },
  { value: "archive", label: "Архів ВЧ", desc: "Одноразовий перенос архіву фахової підготовки" },
];

function DropZone({
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

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    setDragOver(false);
    if (disabled) return;
    const file = e.dataTransfer.files?.[0];
    if (file) onFile(file);
  }

  function handleChange(e: ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
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
        onChange={handleChange}
        className="hidden"
        disabled={disabled}
      />
      <div className="flex flex-col items-center gap-3">
        <div className="icon-box" style={{ width: 48, height: 48 }}>
          {fileName ? (
            <FileUp className="h-6 w-6" />
          ) : (
            <Upload className="h-6 w-6" />
          )}
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
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <Clock className="h-4 w-4" />
          Останні подання
        </CardTitle>
      </CardHeader>
      <CardContent>
        {subs === null ? (
          <div className="flex flex-col gap-2">
            <Skeleton className="h-8 w-full" />
            <Skeleton className="h-8 w-full" />
          </div>
        ) : subs.length === 0 ? (
          <p className="py-6 text-center text-sm text-muted-foreground">
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
      </CardContent>
    </Card>
  );
}

export function ImportPage() {
  const [fileKind, setFileKind] = useState<FileKind>("fah");
  const [fileName, setFileName] = useState<string | null>(null);
  const [file, setFile] = useState<File | null>(null);
  const [uploading, setUploading] = useState(false);
  const [result, setResult] = useState<{ imported: number } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [downloadingTemplate, setDownloadingTemplate] = useState(false);

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
        throw new Error(body.error || `Помилка ${res.status}`);
      }
      const data = await res.json();
      setResult({ imported: data.imported ?? 0 });
      setFile(null);
      setFileName(null);
      toast.success(`Імпортовано ${data.imported ?? 0} записів`);
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Невідома помилка";
      setError(msg);
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

  const kindInfo = FILE_KINDS.find((k) => k.value === fileKind);

  return (
    <div className="flex flex-col gap-6">
      <h1>Імпорт</h1>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <Upload className="h-4 w-4" />
            Імпорт даних з файлів
          </CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col gap-5">
          <div className="flex flex-col gap-1.5">
            <div className="eyebrow" style={{ margin: 0 }}>Тип файлу</div>
            <Select
              value={fileKind}
              onValueChange={(v) => {
                setFileKind(v as FileKind);
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
                {FILE_KINDS.map((k) => (
                  <SelectItem key={k.value} value={k.value}>
                    {k.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            {kindInfo && (
              <p className="text-xs text-muted-foreground">{kindInfo.desc}</p>
            )}
          </div>

          <DropZone fileName={fileName} onFile={handleFile} disabled={uploading} />

          {error && (
            <div
              className="rounded-md border p-3 text-sm"
              style={{
                borderColor: "rgba(220,38,38,0.3)",
                background: "rgba(220,38,38,0.04)",
                color: "#dc2626",
              }}
            >
              {error}
            </div>
          )}

          {result && (
            <div
              className="rounded-md border p-3 text-sm"
              style={{
                borderColor: "rgba(34,197,94,0.3)",
                background: "rgba(34,197,94,0.04)",
                color: "#16a34a",
              }}
            >
              Імпортовано {result.imported} записів
            </div>
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
        </CardContent>
      </Card>

      <Separator />

      <RecentSubmissions />
    </div>
  );
}
