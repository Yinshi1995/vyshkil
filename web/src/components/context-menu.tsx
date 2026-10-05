import { useEffect, useRef, useState, useCallback, type ReactNode } from "react";
import { createPortal } from "react-dom";

export interface ContextMenuItem {
  label: string;
  icon?: ReactNode;
  shortcut?: string;
  variant?: "default" | "destructive";
  disabled?: boolean;
  onClick: () => void;
}

export interface ContextMenuSeparator {
  separator: true;
}

export type ContextMenuEntry = ContextMenuItem | ContextMenuSeparator;

function isSeparator(e: ContextMenuEntry): e is ContextMenuSeparator {
  return "separator" in e;
}

export function useContextMenu() {
  const [state, setState] = useState<{ x: number; y: number; items: ContextMenuEntry[] } | null>(null);

  const open = useCallback((e: React.MouseEvent, items: ContextMenuEntry[]) => {
    e.preventDefault();
    e.stopPropagation();
    setState({ x: e.clientX, y: e.clientY, items });
  }, []);

  const close = useCallback(() => setState(null), []);

  return { state, open, close };
}

export function ContextMenuPortal({
  state,
  onClose,
}: {
  state: { x: number; y: number; items: ContextMenuEntry[] } | null;
  onClose: () => void;
}) {
  const menuRef = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<{ x: number; y: number }>({ x: 0, y: 0 });

  useEffect(() => {
    if (!state) return;
    const el = menuRef.current;
    if (!el) {
      setPos({ x: state.x, y: state.y });
      return;
    }
    requestAnimationFrame(() => {
      const rect = el.getBoundingClientRect();
      let x = state.x;
      let y = state.y;
      if (x + rect.width > window.innerWidth - 8) x = window.innerWidth - rect.width - 8;
      if (y + rect.height > window.innerHeight - 8) y = window.innerHeight - rect.height - 8;
      if (x < 8) x = 8;
      if (y < 8) y = 8;
      setPos({ x, y });
    });
  }, [state]);

  useEffect(() => {
    if (!state) return;
    function handleClick(e: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    }
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    function handleScroll() {
      onClose();
    }
    document.addEventListener("mousedown", handleClick, true);
    document.addEventListener("keydown", handleKey);
    document.addEventListener("scroll", handleScroll, true);
    document.addEventListener("contextmenu", handleClick, true);
    return () => {
      document.removeEventListener("mousedown", handleClick, true);
      document.removeEventListener("keydown", handleKey);
      document.removeEventListener("scroll", handleScroll, true);
      document.removeEventListener("contextmenu", handleClick, true);
    };
  }, [state, onClose]);

  if (!state) return null;

  return createPortal(
    <div
      ref={menuRef}
      className="fixed z-[9999] min-w-[180px] rounded-lg border py-1 shadow-xl animate-in fade-in zoom-in-95 duration-100"
      style={{
        left: pos.x,
        top: pos.y,
        background: "var(--popover)",
        borderColor: "var(--border)",
        boxShadow: "0 8px 30px rgba(0,0,0,0.4), 0 0 0 1px rgba(201,168,76,0.08)",
      }}
    >
      {state.items.map((entry, i) => {
        if (isSeparator(entry)) {
          return (
            <div
              key={`sep-${i}`}
              className="my-1 h-px mx-2"
              style={{ background: "var(--border)" }}
            />
          );
        }
        const item = entry;
        return (
          <button
            key={i}
            disabled={item.disabled}
            className="flex w-full items-center gap-2.5 px-3 py-1.5 text-left text-sm transition-colors duration-100 disabled:opacity-40 disabled:cursor-default"
            style={{
              color: item.variant === "destructive" ? "var(--destructive)" : "var(--popover-foreground)",
            }}
            onMouseEnter={(e) => {
              if (!item.disabled) {
                (e.currentTarget as HTMLElement).style.background = "rgba(201,168,76,0.1)";
              }
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLElement).style.background = "transparent";
            }}
            onClick={() => {
              if (!item.disabled) {
                item.onClick();
                onClose();
              }
            }}
          >
            {item.icon && <span className="flex h-4 w-4 items-center justify-center shrink-0 opacity-70">{item.icon}</span>}
            <span className="flex-1">{item.label}</span>
            {item.shortcut && (
              <span className="text-[10px] opacity-40 ml-4 font-mono">{item.shortcut}</span>
            )}
          </button>
        );
      })}
    </div>,
    document.body,
  );
}
