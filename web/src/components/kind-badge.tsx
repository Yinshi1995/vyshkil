import { useTheme } from "@/context/theme";

const KIND_COLORS: Record<
  string,
  { bg: string; fg: string; darkBg: string; darkFg: string }
> = {
  БЗВП: {
    bg: "rgba(34,139,34,0.10)",
    fg: "#2d7a2d",
    darkBg: "rgba(34,139,34,0.18)",
    darkFg: "#5cb85c",
  },
  Фахова: {
    bg: "rgba(201,168,76,0.10)",
    fg: "#9c8930",
    darkBg: "rgba(201,168,76,0.18)",
    darkFg: "#c9a84c",
  },
  Адаптація: {
    bg: "rgba(100,149,237,0.10)",
    fg: "#4a7ec9",
    darkBg: "rgba(100,149,237,0.18)",
    darkFg: "#6495ed",
  },
  Спеціальна: {
    bg: "rgba(178,102,178,0.10)",
    fg: "#9e5a9e",
    darkBg: "rgba(178,102,178,0.18)",
    darkFg: "#b266b2",
  },
};

export function KindBadge({ kind }: { kind: string }) {
  const { theme } = useTheme();
  const isDark = theme === "dark";
  const c = KIND_COLORS[kind];
  const bg = c ? (isDark ? c.darkBg : c.bg) : "rgba(138,133,119,0.1)";
  const fg = c ? (isDark ? c.darkFg : c.fg) : "var(--muted-foreground)";
  return (
    <span
      className="inline-flex items-center rounded-md px-2 py-0.5 text-xs font-semibold whitespace-nowrap"
      style={{ background: bg, color: fg, letterSpacing: "0.02em" }}
    >
      {kind || "—"}
    </span>
  );
}
