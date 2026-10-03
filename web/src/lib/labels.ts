const SOURCE_TYPE: Record<string, string> = {
  form: "Форма",
  table: "Таблиця",
  official_letter: "Офіц. лист",
};

const STATUS: Record<string, string> = {
  committed: "Зафіксовано",
  draft: "Чернетка",
  rejected: "Відхилено",
};

export function sourceTypeLabel(raw: string): string {
  return SOURCE_TYPE[raw] ?? raw;
}

export function statusLabel(raw: string): string {
  return STATUS[raw] ?? raw;
}

export function statusVariant(
  raw: string,
): "default" | "secondary" | "destructive" | "outline" {
  switch (raw) {
    case "committed":
      return "default";
    case "draft":
      return "secondary";
    case "rejected":
      return "destructive";
    default:
      return "outline";
  }
}
