const DATE_RE = /^\d{2}\.\d{2}\.\d{4}$/;

export function parseUaDate(s: string): Date | null {
  if (!DATE_RE.test(s)) return null;
  const [dd, mm, yyyy] = s.split(".");
  const d = new Date(+yyyy, +mm - 1, +dd);
  if (d.getDate() !== +dd || d.getMonth() !== +mm - 1) return null;
  return d;
}

export function toIso(uaDate: string): string {
  const d = parseUaDate(uaDate);
  if (!d) return "";
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${dd}`;
}

export function fromIso(iso: string): string {
  if (!iso) return "";
  const [y, m, d] = iso.split("-");
  if (!y || !m || !d) return iso;
  return `${d}.${m}.${y}`;
}

export function todayUa(): string {
  const d = new Date();
  const dd = String(d.getDate()).padStart(2, "0");
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  return `${dd}.${mm}.${d.getFullYear()}`;
}

export function todayIso(): string {
  const d = new Date();
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${mm}-${dd}`;
}
