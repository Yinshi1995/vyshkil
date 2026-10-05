import { useCallback, useEffect, useState } from "react";
import { Link, useLocation, useNavigate } from "react-router-dom";
import { useAuth } from "@/context/auth";
import { useTheme } from "@/context/theme";
import { api } from "@/api/client";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { Sheet, SheetContent, SheetTrigger } from "@/components/ui/sheet";
import {
  Menu,
  ChevronDown,
  LogOut,
  Building2,
  FileText,
  FileCheck,
  AlertTriangle,
  Settings,
  Table,
  Moon,
  Sun,
  Bell,
  Check,
  Upload,
  Users as UsersIcon,
  MessageCircle,
  BarChart3,
  MoreHorizontal,
} from "lucide-react";

interface NotificationItem {
  id: number;
  kind: string;
  title: string;
  body: string | null;
  link: string | null;
  is_read: boolean;
  created_at: string;
}

interface NavItem {
  href: string;
  label: string;
  icon: React.ElementType;
  adminOnly?: boolean;
}

const PRIMARY_NAV: NavItem[] = [
  { href: "/orgs", label: "Підрозділи", icon: Building2, adminOnly: true },
  { href: "/training", label: "Підготовка", icon: FileText },
  { href: "/dashboard", label: "Аналітика", icon: BarChart3 },
  { href: "/discrepancies", label: "Розбіжності", icon: AlertTriangle },
  { href: "/chat", label: "Чат", icon: MessageCircle },
];

const MORE_NAV: NavItem[] = [
  { href: "/data", label: "Дані", icon: Table },
  { href: "/documents", label: "Документи", icon: FileCheck },
  { href: "/import", label: "Імпорт", icon: Upload, adminOnly: true },
  { href: "/directory", label: "Люди", icon: UsersIcon },
];

const ALL_NAV = [...PRIMARY_NAV, ...MORE_NAV];

const navLinkStyle = (active: boolean) => ({
  fontFamily: "var(--font-heading)",
  fontSize: "13px",
  fontWeight: 700 as const,
  textTransform: "uppercase" as const,
  letterSpacing: "0.05em",
  color: active ? "var(--primary)" : "var(--muted-foreground)",
  padding: "8px 4px",
  position: "relative" as const,
  transition: "color 0.15s ease",
  textDecoration: "none" as const,
});

function ActiveUnderline() {
  return (
    <span
      className="animate-in fade-in slide-in-from-left-2 duration-300"
      style={{
        position: "absolute",
        bottom: 0,
        left: 0,
        right: 0,
        height: "2px",
        background: "var(--primary)",
        borderRadius: "1px",
        boxShadow: "0 0 8px rgba(201, 168, 76, 0.4)",
      }}
    />
  );
}

export function Header() {
  const { user, actor, isAdmin, logout } = useAuth();
  const { theme, toggle: toggleTheme } = useTheme();
  const location = useLocation();
  const navigate = useNavigate();
  const [mobileOpen, setMobileOpen] = useState(false);
  const [notifications, setNotifications] = useState<NotificationItem[]>([]);
  const [unreadCount, setUnreadCount] = useState(0);

  const loadNotifications = useCallback(() => {
    if (!actor) return;
    api.get<NotificationItem[]>("/notifications?limit=20")
      .then((items) => {
        setNotifications(items);
        setUnreadCount(items.filter((n) => !n.is_read).length);
      })
      .catch(() => {});
  }, [actor]);

  useEffect(() => {
    loadNotifications();
    const interval = setInterval(loadNotifications, 30_000);
    return () => clearInterval(interval);
  }, [loadNotifications]);

  const markRead = useCallback(async (id: number) => {
    try {
      await api.post(`/notifications/${id}/read`, {});
      setNotifications((prev) =>
        prev.map((n) => (n.id === id ? { ...n, is_read: true } : n)),
      );
      setUnreadCount((c) => Math.max(0, c - 1));
    } catch {}
  }, []);

  const markAllRead = useCallback(async () => {
    try {
      await api.post("/notifications/read-all", {});
      setNotifications((prev) => prev.map((n) => ({ ...n, is_read: true })));
      setUnreadCount(0);
    } catch {}
  }, []);

  const visibleMore = MORE_NAV.filter((n) => !n.adminOnly || isAdmin);
  const moreIsActive = visibleMore.some((n) => location.pathname === n.href);
  const allVisible = ALL_NAV.filter((n) => !n.adminOnly || isAdmin);
  const orgHref = actor ? `/org/${actor.org_id}` : null;

  return (
    <header className="sticky top-0 z-50 border-b border-border backdrop-blur app-header">
      <div className="mx-auto flex h-14 max-w-7xl items-center gap-4 px-4">
        {/* Brand — doubles as "Головна" */}
        <Link to="/" className="flex items-center gap-3">
          <img
            src="/emblem.png"
            alt="Вишкіл"
            className="h-9 w-9 object-contain"
            style={{ filter: "drop-shadow(0 0 6px rgba(201,168,76,0.3))" }}
          />
          <span
            className="hidden font-bold text-foreground sm:inline"
            style={{
              fontFamily: "var(--font-heading)",
              textTransform: "uppercase",
              letterSpacing: "0.08em",
              fontSize: "19px",
            }}
          >
            Вишкіл
          </span>
        </Link>

        {/* Primary nav */}
        <nav className="ml-6 mr-auto hidden items-center gap-4 md:flex">
          {PRIMARY_NAV.filter((n) => !n.adminOnly || isAdmin).map((item) => {
            const active = location.pathname === item.href;
            return (
              <Link
                key={item.href}
                to={item.href}
                style={navLinkStyle(active)}
                onMouseEnter={(e) => { if (!active) e.currentTarget.style.color = "var(--primary)"; }}
                onMouseLeave={(e) => { if (!active) e.currentTarget.style.color = "var(--muted-foreground)"; }}
              >
                {item.label}
                {active && <ActiveUnderline />}
              </Link>
            );
          })}

          {/* "Ще" dropdown */}
          <DropdownMenu>
            <DropdownMenuTrigger
              className="flex items-center gap-1 outline-none"
              style={navLinkStyle(moreIsActive)}
              onMouseEnter={(e) => { if (!moreIsActive) e.currentTarget.style.color = "var(--primary)"; }}
              onMouseLeave={(e) => { if (!moreIsActive) e.currentTarget.style.color = "var(--muted-foreground)"; }}
            >
              <MoreHorizontal className="h-4 w-4" />
              Ще
              <ChevronDown className="h-3 w-3 opacity-50" />
              {moreIsActive && <ActiveUnderline />}
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start" className="w-44">
              {visibleMore.map((item) => {
                const Icon = item.icon;
                const active = location.pathname === item.href;
                return (
                  <DropdownMenuItem
                    key={item.href}
                    onClick={() => navigate(item.href)}
                    className="gap-2"
                    style={{
                      fontFamily: "var(--font-heading)",
                      fontSize: "13px",
                      fontWeight: 600,
                      textTransform: "uppercase",
                      letterSpacing: "0.04em",
                      color: active ? "var(--primary)" : undefined,
                    }}
                  >
                    <Icon className="h-4 w-4" />
                    {item.label}
                  </DropdownMenuItem>
                );
              })}
              {orgHref && (
                <>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem
                    onClick={() => navigate(orgHref)}
                    className="gap-2"
                    style={{
                      fontFamily: "var(--font-heading)",
                      fontSize: "13px",
                      fontWeight: 600,
                      textTransform: "uppercase",
                      letterSpacing: "0.04em",
                      color: location.pathname.startsWith("/org/") ? "var(--primary)" : undefined,
                    }}
                  >
                    <Building2 className="h-4 w-4" />
                    Мій підрозділ
                  </DropdownMenuItem>
                </>
              )}
            </DropdownMenuContent>
          </DropdownMenu>
        </nav>

        {/* Right side */}
        <div className="ml-auto flex items-center gap-3 md:ml-0">
          {/* Notifications bell */}
          {actor && (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  variant="outline"
                  size="icon"
                  className="relative h-8 w-8"
                  title="Сповіщення"
                  aria-label={`Сповіщення${unreadCount > 0 ? `, ${unreadCount} непрочитаних` : ""}`}
                >
                  <Bell className="h-4 w-4" />
                  {unreadCount > 0 && (
                    <span
                      className="absolute -right-1 -top-1 flex h-4 min-w-4 items-center justify-center rounded-full px-1 text-[10px] font-bold"
                      style={{ background: "#D9534F", color: "#fff" }}
                    >
                      {unreadCount > 9 ? "9+" : unreadCount}
                    </span>
                  )}
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-80 max-h-96 overflow-y-auto">
                <div className="flex items-center justify-between px-3 py-2">
                  <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                    Сповіщення
                  </span>
                  {unreadCount > 0 && (
                    <Button
                      variant="link"
                      size="sm"
                      className="h-auto p-0 text-[11px]"
                      onClick={(e) => { e.stopPropagation(); markAllRead(); }}
                    >
                      Прочитати все
                    </Button>
                  )}
                </div>
                <DropdownMenuSeparator />
                {notifications.length === 0 ? (
                  <div className="px-3 py-6 text-center text-sm text-muted-foreground">
                    Немає сповіщень
                  </div>
                ) : (
                  notifications.slice(0, 15).map((n) => (
                    <DropdownMenuItem
                      key={n.id}
                      className="flex flex-col items-start gap-0.5 py-2"
                      onClick={() => {
                        if (!n.is_read) markRead(n.id);
                        if (n.link) navigate(n.link);
                      }}
                    >
                      <div className="flex w-full items-center gap-2">
                        {!n.is_read && (
                          <span className="h-2 w-2 shrink-0 rounded-full" style={{ background: "#5B9BD5" }} />
                        )}
                        <span className={`flex-1 text-sm ${n.is_read ? "text-muted-foreground" : "font-medium"}`}>
                          {n.title}
                        </span>
                        {!n.is_read && (
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-5 w-5 shrink-0"
                            onClick={(e) => { e.stopPropagation(); markRead(n.id); }}
                            title="Позначити прочитаним"
                          >
                            <Check className="h-3 w-3" />
                          </Button>
                        )}
                      </div>
                      {n.body && (
                        <span className="text-xs text-muted-foreground line-clamp-2">{n.body}</span>
                      )}
                      <span className="text-[10px] text-muted-foreground">
                        {n.created_at.replace("T", " ")}
                      </span>
                    </DropdownMenuItem>
                  ))
                )}
              </DropdownMenuContent>
            </DropdownMenu>
          )}

          <Button
            variant="outline"
            size="icon"
            className="h-8 w-8"
            onClick={toggleTheme}
            title={theme === "dark" ? "Денна тема" : "Нічна тема"}
            aria-label={theme === "dark" ? "Увімкнути денну тему" : "Увімкнути нічну тему"}
          >
            {theme === "dark" ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
          </Button>
          {user && actor && (
            <DropdownMenu>
              <DropdownMenuTrigger
                className="flex items-center gap-2 rounded-md border border-border px-2 py-1"
                style={{
                  fontFamily: "var(--font-heading)",
                  fontSize: "13px",
                  fontWeight: 500,
                  textTransform: "uppercase",
                  letterSpacing: "0.03em",
                  background: "none",
                  color: "var(--foreground)",
                  cursor: "pointer",
                  transition: "border-color 0.15s ease",
                }}
              >
                {user.avatar_url ? (
                  <img
                    src={user.avatar_url}
                    alt=""
                    className="h-[26px] w-[26px] rounded-full object-cover"
                  />
                ) : (
                  <span
                    className="flex h-[26px] w-[26px] items-center justify-center rounded-full text-[13px] font-bold"
                    style={{ background: "var(--primary)", color: "var(--primary-foreground)" }}
                  >
                    {(user.callsign ?? user.display_name ?? "U")[0]}
                  </span>
                )}
                <span className="max-w-[140px] truncate">{user.callsign ?? actor.org_label}</span>
                <ChevronDown className="h-3 w-3 opacity-50" />
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-56 p-1.5">
                <div className="flex items-center gap-3 px-2 py-2.5">
                  {user.avatar_url ? (
                    <img src={user.avatar_url} alt="" className="h-10 w-10 rounded-full object-cover shrink-0" />
                  ) : (
                    <span
                      className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full text-base font-bold"
                      style={{ background: "var(--primary)", color: "var(--primary-foreground)" }}
                    >
                      {(user.callsign ?? user.display_name ?? "U")[0]}
                    </span>
                  )}
                  <div className="min-w-0">
                    <div className="text-sm font-semibold truncate">
                      {user.callsign ?? user.display_name ?? String(user.user_id)}
                    </div>
                    <div className="text-xs text-muted-foreground truncate">
                      {actor.role} — {actor.org_label}
                    </div>
                  </div>
                </div>
                <DropdownMenuSeparator />
                <DropdownMenuItem className="py-2 text-sm" onClick={() => navigate("/settings")}>
                  <Settings className="mr-2.5 h-4 w-4" />
                  Налаштування
                </DropdownMenuItem>
                <DropdownMenuItem className="py-2 text-sm text-red-400" onClick={() => logout()}>
                  <LogOut className="mr-2.5 h-4 w-4" />
                  Вихід
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          )}

          {/* Mobile hamburger */}
          <Sheet open={mobileOpen} onOpenChange={setMobileOpen}>
            <SheetTrigger className="inline-flex items-center justify-center rounded-lg p-2 text-muted-foreground hover:text-primary md:hidden" aria-label="Відкрити меню">
              <Menu className="h-5 w-5" />
            </SheetTrigger>
            <SheetContent side="left" className="w-64 p-0">
              <nav className="flex flex-col gap-1 p-4 pt-12">
                {allVisible.map((item) => {
                  const Icon = item.icon;
                  const active = location.pathname === item.href;
                  return (
                    <Link
                      key={item.href}
                      to={item.href}
                      onClick={() => setMobileOpen(false)}
                      className="flex items-center gap-2 rounded-md px-3 py-2 text-sm transition-colors"
                      style={{
                        fontFamily: "var(--font-heading)",
                        textTransform: "uppercase",
                        letterSpacing: "0.05em",
                        fontWeight: 700,
                        fontSize: "13px",
                        color: active ? "var(--primary)" : "var(--muted-foreground)",
                        background: active ? "color-mix(in srgb, var(--primary) 8%, transparent)" : "transparent",
                        textDecoration: "none",
                      }}
                    >
                      <Icon className="h-4 w-4" />
                      {item.label}
                    </Link>
                  );
                })}
                {orgHref && (
                  <Link
                    to={orgHref}
                    onClick={() => setMobileOpen(false)}
                    className="flex items-center gap-2 rounded-md px-3 py-2 text-sm"
                    style={{
                      fontFamily: "var(--font-heading)",
                      textTransform: "uppercase",
                      letterSpacing: "0.05em",
                      fontWeight: 700,
                      fontSize: "13px",
                      color: "var(--muted-foreground)",
                      textDecoration: "none",
                    }}
                  >
                    <Building2 className="h-4 w-4" />
                    Мій підрозділ
                  </Link>
                )}
              </nav>
            </SheetContent>
          </Sheet>
        </div>
      </div>
    </header>
  );
}
