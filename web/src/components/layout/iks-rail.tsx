import { useState, type CSSProperties } from "react";
import { House, LayoutGrid, LogOut, UserRound } from "lucide-react";
import { useAuth } from "@/context/auth";
import { useTheme } from "@/context/theme";
import { iksUrls, useIksModules, type IksModule } from "@/lib/iks";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLinkItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Sheet, SheetContent, SheetTitle, SheetTrigger } from "@/components/ui/sheet";

/*
 * Рейка ІКС (docs/spec/components/iks-rail.md, контракт — pult/docs/IKS-RAIL.md).
 * Окремо від навігації Вишколу: ≥1024 px — вузька рейка зліва, вужче — кнопка з гербом у шапці.
 */

function Emblem({ className }: { className?: string }) {
  const [failed, setFailed] = useState(false);
  if (!iksUrls || failed) return <LayoutGrid className={className} aria-hidden />;
  return (
    <img
      src={iksUrls.emblem}
      alt=""
      className={className}
      draggable={false}
      onError={() => setFailed(true)}
    />
  );
}

function ModuleIcon({ module }: { module: IksModule }) {
  const { theme } = useTheme();
  const [failed, setFailed] = useState(false);
  if (!iksUrls || failed) {
    return <span className="iks-letter">{module.title.charAt(0).toUpperCase()}</span>;
  }
  return (
    <img
      src={iksUrls.icon(module.name, theme)}
      alt=""
      className="iks-icon"
      draggable={false}
      onError={() => setFailed(true)}
    />
  );
}

function userName(user: { callsign: string | null; display_name: string | null; user_id: number }) {
  return user.callsign ?? user.display_name ?? String(user.user_id);
}

function UserAvatar({ size }: { size: number }) {
  const { user } = useAuth();
  if (!user) return null;
  const style = { width: size, height: size };
  return user.avatar_url ? (
    <img src={user.avatar_url} alt="" className="shrink-0 rounded-full object-cover" style={style} />
  ) : (
    <span
      className="flex shrink-0 items-center justify-center rounded-full font-bold"
      style={{ ...style, fontSize: size * 0.4, background: "var(--primary)", color: "var(--primary-foreground)" }}
    >
      {userName(user).charAt(0).toUpperCase()}
    </span>
  );
}

/* ---------------------------------------------------------------- desktop */

function RailModule({ module, index }: { module: IksModule; index: number }) {
  return (
    <Tooltip>
      <TooltipTrigger
        delay={120}
        render={
          <a
            href={module.url}
            className="iks-item iks-enter"
            aria-current={module.current ? "page" : undefined}
            aria-label={module.title}
            style={{ "--i": index + 1 } as CSSProperties}
          />
        }
      >
        <span className="iks-pill" aria-hidden />
        <span className="iks-tile">
          <ModuleIcon module={module} />
        </span>
      </TooltipTrigger>
      <TooltipContent side="right" sideOffset={6} className="iks-tip">
        <span className="iks-tip-title">{module.title}</span>
        {module.description && <span className="iks-tip-desc">{module.description}</span>}
      </TooltipContent>
    </Tooltip>
  );
}

function AccountMenu() {
  const { user } = useAuth();
  if (!user || !iksUrls) return null;
  const name = userName(user);
  const sub = user.callsign && user.display_name ? user.display_name : null;
  return (
    <DropdownMenu>
      <DropdownMenuTrigger className="iks-avatar iks-enter" aria-label="Профіль ІКС" style={{ "--i": 0 } as CSSProperties}>
        <UserAvatar size={36} />
      </DropdownMenuTrigger>
      <DropdownMenuContent side="right" align="end" sideOffset={14} className="w-60 p-1.5">
        <div className="flex items-center gap-3 px-2 py-2.5">
          <UserAvatar size={40} />
          <div className="min-w-0">
            <div className="truncate text-sm font-semibold">{name}</div>
            {sub && <div className="truncate text-xs text-muted-foreground">{sub}</div>}
          </div>
        </div>
        <DropdownMenuSeparator />
        <DropdownMenuLinkItem href={iksUrls.profile} className="gap-2.5 py-2">
          <UserRound />
          Мій профіль ІКС
        </DropdownMenuLinkItem>
        <DropdownMenuLinkItem href={iksUrls.home} className="gap-2.5 py-2">
          <House />
          Головна
        </DropdownMenuLinkItem>
        <DropdownMenuSeparator />
        <DropdownMenuLinkItem href={iksUrls.logoutAll} variant="destructive" className="gap-2.5 py-2">
          <LogOut />
          Вийти з усіх застосунків
        </DropdownMenuLinkItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** Вузька рейка зліва (лише ≥1024 px). */
export function IksRail() {
  const modules = useIksModules();
  if (!iksUrls) return null;
  return (
    <aside className="iks-rail" aria-label="ІКС">
      <Tooltip>
        <TooltipTrigger
          delay={120}
          render={<a href={iksUrls.home} className="iks-emblem iks-enter" aria-label="Головна ІКС" style={{ "--i": 0 } as CSSProperties} />}
        >
          <Emblem className="size-9 object-contain" />
        </TooltipTrigger>
        <TooltipContent side="right" sideOffset={10} className="iks-tip">
          <span className="iks-tip-title">Головна ІКС</span>
        </TooltipContent>
      </Tooltip>

      <div className="iks-rule" aria-hidden />

      <nav className="flex w-full flex-col gap-1.5" aria-label="Застосунки ІКС">
        {modules.map((m, i) => (
          <RailModule key={m.name} module={m} index={i} />
        ))}
      </nav>

      <div className="flex-1" />
      <AccountMenu />
    </aside>
  );
}

/* ----------------------------------------------------------------- mobile */

/** Кнопка з гербом ІКС у шапці (лише < 1024 px) → панель із тим самим, що в рейці. */
export function IksMobileButton() {
  const { user } = useAuth();
  const modules = useIksModules();
  const [open, setOpen] = useState(false);
  if (!iksUrls) return null;
  return (
    <Sheet open={open} onOpenChange={setOpen}>
      {/* Не герб: поруч — герб Вишколу, два майже однакові щити читаються як повтор. Перемикач
          модулів (сітка) — того ж вигляду, що й інші кнопки шапки. */}
      <SheetTrigger className="iks-mobile-trigger lg:hidden" aria-label="Модулі ІКС" title="Модулі ІКС">
        <LayoutGrid className="size-4" />
      </SheetTrigger>
      <SheetContent side="left" className="iks-sheet gap-0 p-0 max-sm:top-auto sm:w-[22rem]">
        <a href={iksUrls.home} className="iks-sheet-home">
          <Emblem className="size-10 object-contain" />
          <span className="min-w-0">
            <SheetTitle className="iks-sheet-title">ІКС</SheetTitle>
            <span className="text-xs text-muted-foreground">Головна</span>
          </span>
        </a>

        {modules.length > 0 && (
          <nav className="iks-sheet-grid" aria-label="Застосунки ІКС">
            {modules.map((m, i) => (
              <a
                key={m.name}
                href={m.url}
                className="iks-sheet-module iks-enter"
                aria-current={m.current ? "page" : undefined}
                style={{ "--i": i + 1 } as CSSProperties}
              >
                <span className="iks-tile">
                  <ModuleIcon module={m} />
                </span>
                <span className="iks-sheet-label">{m.title}</span>
              </a>
            ))}
          </nav>
        )}

        <div className="mt-auto border-t border-border p-3">
          {user && (
            <a href={iksUrls.profile} className="iks-sheet-row">
              <UserAvatar size={36} />
              <span className="min-w-0 flex-1">
                <span className="block truncate text-sm font-semibold text-foreground">{userName(user)}</span>
                <span className="block text-xs text-muted-foreground">Мій профіль ІКС</span>
              </span>
              <UserRound className="size-4 text-muted-foreground" aria-hidden />
            </a>
          )}
          <a href={iksUrls.logoutAll} className="iks-sheet-row iks-sheet-danger">
            <LogOut className="size-4" aria-hidden />
            Вийти з усіх застосунків
          </a>
        </div>
      </SheetContent>
    </Sheet>
  );
}
