import { useEffect, useState } from "react";
import { api } from "@/api/client";
import type { AdminGroupRow } from "@/api/types";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { FileText, Users, Upload } from "lucide-react";

const FILE_KINDS = [
  { key: "fah", label: "Фах (Пройшли/Проходять)" },
  { key: "bps", label: "БпС (Завершилась/Навчаються)" },
  { key: "terminy", label: "Терміни (БЗВП/Фахова/Адаптація)" },
] as const;

function StatBadge({ label, value }: { label: string; value: number }) {
  return (
    <div className="flex items-center gap-2">
      <span
        style={{
          fontFamily: "var(--font-heading)",
          textTransform: "uppercase",
          fontSize: "11px",
          fontWeight: 700,
          letterSpacing: "0.12em",
          color: "#8a8577",
        }}
      >
        {label}
      </span>
      <span className="stat-value" style={{ fontSize: "20px" }}>
        {value}
      </span>
    </div>
  );
}

export function TrainingFormPage() {
  const [groups, setGroups] = useState<AdminGroupRow[] | null>(null);
  const [selectedKind, setSelectedKind] = useState("fah");

  useEffect(() => {
    api
      .get<AdminGroupRow[]>("/training/groups")
      .then(setGroups)
      .catch(() => setGroups([]));
  }, []);

  const totalPlanned = groups?.reduce((s, g) => s + g.planned_count, 0) ?? 0;
  const totalArrived = groups?.reduce((s, g) => s + g.arrived_count, 0) ?? 0;
  const totalTraining = groups?.reduce((s, g) => s + g.in_training_count, 0) ?? 0;

  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-bold">Форма подання</h1>

      {/* Statistics */}
      {groups !== null && groups.length > 0 && (
        <div className="flex flex-wrap gap-6">
          <StatBadge label="Груп" value={groups.length} />
          <StatBadge label="План" value={totalPlanned} />
          <StatBadge label="Прибуло" value={totalArrived} />
          <StatBadge label="Навчається" value={totalTraining} />
        </div>
      )}

      {/* File import section */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <Upload className="h-4 w-4" />
            Додати з файлу
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex flex-col gap-4">
            <div className="flex flex-wrap gap-2">
              {FILE_KINDS.map((fk) => (
                <button
                  key={fk.key}
                  onClick={() => setSelectedKind(fk.key)}
                  className="rounded-md border px-3 py-1.5 text-sm transition-colors"
                  style={{
                    fontFamily: "var(--font-heading)",
                    fontWeight: 600,
                    textTransform: "uppercase",
                    fontSize: "12px",
                    letterSpacing: "0.04em",
                    borderColor:
                      selectedKind === fk.key
                        ? "var(--primary)"
                        : "var(--border)",
                    color:
                      selectedKind === fk.key
                        ? "var(--primary)"
                        : "var(--muted-foreground)",
                    background:
                      selectedKind === fk.key
                        ? "rgba(201,168,76,0.08)"
                        : "transparent",
                  }}
                >
                  {fk.label}
                </button>
              ))}
            </div>
            <div
              className="flex items-center justify-center rounded-lg border-2 border-dashed px-6 py-8 text-center"
              style={{ borderColor: "var(--border)" }}
            >
              <div className="flex flex-col items-center gap-2">
                <FileText
                  className="h-8 w-8"
                  style={{ color: "var(--muted-foreground)" }}
                />
                <p className="text-sm text-muted-foreground">
                  Перетягніть .xlsx файл сюди або натисніть для вибору
                </p>
                <Badge variant="outline" className="text-xs">
                  {FILE_KINDS.find((fk) => fk.key === selectedKind)?.label}
                </Badge>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      <Separator />

      {/* Groups table */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <Users className="h-4 w-4" />
            Групи на навчанні
            {groups !== null && (
              <Badge variant="secondary" className="ml-1">
                {groups.length}
              </Badge>
            )}
          </CardTitle>
        </CardHeader>
        <CardContent>
          {groups === null ? (
            <div className="flex flex-col gap-2">
              <Skeleton className="h-8 w-full" />
              <Skeleton className="h-8 w-full" />
              <Skeleton className="h-8 w-full" />
              <Skeleton className="h-8 w-full" />
            </div>
          ) : groups.length === 0 ? (
            <p className="py-8 text-center text-sm text-muted-foreground">
              Груп підготовки немає
            </p>
          ) : (
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Підрозділ</TableHead>
                    <TableHead>Вид</TableHead>
                    <TableHead>ВОС</TableHead>
                    <TableHead>Полігон</TableHead>
                    <TableHead>Початок</TableHead>
                    <TableHead>Кінець</TableHead>
                    <TableHead className="text-right">План</TableHead>
                    <TableHead className="text-right">Прибули</TableHead>
                    <TableHead className="text-right">Навч.</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {groups.map((g) => (
                    <TableRow key={g.id}>
                      <TableCell className="font-medium">
                        {g.org_label}
                      </TableCell>
                      <TableCell>
                        <Badge variant="outline">{g.training_kind}</Badge>
                      </TableCell>
                      <TableCell className="text-sm">{g.vos_label}</TableCell>
                      <TableCell className="text-sm">{g.site_label}</TableCell>
                      <TableCell>{g.planned_start}</TableCell>
                      <TableCell>{g.planned_end}</TableCell>
                      <TableCell className="text-right font-medium">
                        {g.planned_count}
                      </TableCell>
                      <TableCell className="text-right">
                        {g.arrived_count}
                      </TableCell>
                      <TableCell className="text-right">
                        {g.in_training_count}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
