import { useEffect, useState, useCallback } from "react";
import { useSearchParams, useNavigate } from "react-router-dom";
import { useAuth } from "@/context/auth";
import { api } from "@/api/client";
import type { DirectoryUser } from "@/api/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { toast } from "sonner";
import {
  Users as UsersIcon,
  Shield,
  Phone,
  MessageCircle,
  User,
  Search as SearchIcon,
  KeyRound,
  ToggleLeft,
  ToggleRight,
  Loader2,
  UserX,
  UserCheck,
  MessageSquarePlus,
} from "lucide-react";

const ROLE_UA: Record<string, string> = {
  admin: "Адмін",
  org_editor: "Редактор",
  viewer: "Переглядач",
};

const ROLE_COLORS: Record<string, string> = {
  admin: "var(--primary)",
  org_editor: "#5B9BD5",
  viewer: "#8FB339",
};

function UserCard({
  user,
  index,
  isAdmin,
  isSelf,
  onToggleActive,
  onResetPassword,
  onMessage,
}: {
  user: DirectoryUser;
  index: number;
  isAdmin: boolean;
  isSelf: boolean;
  onToggleActive: (userId: number) => void;
  onResetPassword: (userId: number) => void;
  onMessage: (userId: number) => void;
}) {
  const name = user.callsign ?? user.display_name ?? user.login;
  const fullName = [user.rank, user.first_name, user.last_name].filter(Boolean).join(" ");
  const roleColor = ROLE_COLORS[user.role] ?? "var(--muted-foreground)";
  const initials = name.slice(0, 2).toUpperCase();
  const canManage = isAdmin && user.role !== "admin";
  const contactInfo = [
    user.phone ? { icon: Phone, text: user.phone } : null,
    user.delta_nick ? { icon: MessageCircle, text: `IXD: ${user.delta_nick}` } : null,
  ].filter(Boolean) as { icon: typeof Phone; text: string }[];

  return (
    <div
      className="group relative rounded-xl border border-border/50 p-4 transition-all duration-300 hover:border-primary/40 hover:shadow-lg hover:shadow-primary/5 hover:-translate-y-0.5"
      style={{
        background: "var(--card)",
        animationDelay: `${index * 60}ms`,
        animationFillMode: "both",
        opacity: user.is_active ? 1 : 0.5,
      }}
    >
      <div className="flex items-start gap-3">
        <div className="relative shrink-0">
          <div
            className="h-12 w-12 rounded-full overflow-hidden flex items-center justify-center text-sm font-bold transition-transform duration-300 group-hover:scale-105"
            style={{
              background: user.avatar_url
                ? "transparent"
                : `linear-gradient(135deg, ${roleColor}, color-mix(in oklch, ${roleColor} 60%, transparent))`,
              color: "var(--primary-foreground)",
            }}
          >
            {user.avatar_url ? (
              <img src={user.avatar_url} alt="" className="h-full w-full object-cover" />
            ) : (
              initials
            )}
          </div>
          <div
            className="absolute -bottom-0.5 -right-0.5 h-3.5 w-3.5 rounded-full border-2"
            style={{ borderColor: "var(--card)", background: user.is_active ? roleColor : "var(--muted-foreground)" }}
          />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <p className="font-semibold text-sm truncate">{name}</p>
            {!user.is_active && (
              <Badge variant="destructive" className="text-[10px] px-1.5 py-0 shrink-0">
                Заблокований
              </Badge>
            )}
          </div>
          {fullName && <p className="text-xs text-muted-foreground truncate">{fullName}</p>}
          <div className="flex items-center gap-2 mt-1.5">
            <Badge
              variant="outline"
              className="text-[10px] px-1.5 py-0"
              style={{ borderColor: `color-mix(in oklch, ${roleColor} 40%, transparent)`, color: roleColor }}
            >
              {ROLE_UA[user.role] ?? user.role}
            </Badge>
            <span className="text-[11px] text-muted-foreground truncate">{user.org_label}</span>
          </div>
          {contactInfo.length > 0 && (
            <div className="flex flex-col gap-0.5 mt-2 pt-2 border-t border-border/40">
              {contactInfo.map((c, i) => (
                <div key={i} className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
                  <c.icon className="h-3 w-3 shrink-0" />
                  <span className="truncate">{c.text}</span>
                </div>
              ))}
            </div>
          )}
          {(!isSelf && user.is_active || canManage) && (
            <div className="flex items-center gap-1 mt-2 pt-2 border-t border-border/40 sm:opacity-0 sm:group-hover:opacity-100 transition-opacity duration-200">
              {!isSelf && user.is_active && (
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-6 px-1.5 sm:px-2 text-[11px]"
                  onClick={() => onMessage(user.user_id)}
                  title="Написати"
                >
                  <MessageSquarePlus className="h-3.5 w-3.5 sm:mr-1 sm:h-3 sm:w-3" />
                  <span className="hidden sm:inline">Написати</span>
                </Button>
              )}
              {canManage && (
                <>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-6 px-1.5 sm:px-2 text-[11px]"
                    onClick={() => onToggleActive(user.user_id)}
                    title={user.is_active ? "Заблокувати" : "Розблокувати"}
                  >
                    {user.is_active ? (
                      <><UserX className="h-3.5 w-3.5 sm:mr-1 sm:h-3 sm:w-3" /><span className="hidden sm:inline">Заблокувати</span></>
                    ) : (
                      <><UserCheck className="h-3.5 w-3.5 sm:mr-1 sm:h-3 sm:w-3" /><span className="hidden sm:inline">Розблокувати</span></>
                    )}
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-6 px-1.5 sm:px-2 text-[11px]"
                    onClick={() => onResetPassword(user.user_id)}
                    title="Скинути пароль"
                  >
                    <KeyRound className="h-3.5 w-3.5 sm:mr-1 sm:h-3 sm:w-3" />
                    <span className="hidden sm:inline">Пароль</span>
                  </Button>
                </>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export function DirectoryPage() {
  const { isAdmin, user: authUser } = useAuth();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const [users, setUsers] = useState<DirectoryUser[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState(searchParams.get("search") ?? "");
  const [showInactive, setShowInactive] = useState(false);
  const [resetDialogUser, setResetDialogUser] = useState<number | null>(null);
  const [newPassword, setNewPassword] = useState("");
  const [resetting, setResetting] = useState(false);

  const loadUsers = useCallback(() => {
    const params = showInactive ? "?include_inactive=true" : "";
    api.get<DirectoryUser[]>(`/directory${params}`)
      .then(setUsers)
      .catch(() => {})
      .finally(() => setLoading(false));
  }, [showInactive]);

  useEffect(() => {
    setLoading(true);
    loadUsers();
  }, [loadUsers]);

  const filtered = search
    ? users.filter((u) => {
        const q = search.toLowerCase();
        return (
          u.login.toLowerCase().includes(q) ||
          (u.display_name ?? "").toLowerCase().includes(q) ||
          (u.callsign ?? "").toLowerCase().includes(q) ||
          (u.first_name ?? "").toLowerCase().includes(q) ||
          (u.last_name ?? "").toLowerCase().includes(q) ||
          (u.delta_nick ?? "").toLowerCase().includes(q) ||
          u.org_label.toLowerCase().includes(q)
        );
      })
    : users;

  async function handleToggleActive(userId: number) {
    try {
      await api.post(`/admin/users/${userId}/toggle-active`);
      toast.success("Статус змінено");
      loadUsers();
    } catch {
      toast.error("Немає доступу або помилка");
    }
  }

  async function handleResetPassword() {
    if (!resetDialogUser || !newPassword) return;
    setResetting(true);
    try {
      await api.post(`/admin/users/${resetDialogUser}/reset-password`, { temp_password: newPassword });
      toast.success("Пароль скинуто");
      setResetDialogUser(null);
      setNewPassword("");
    } catch {
      toast.error("Немає доступу або помилка");
    } finally {
      setResetting(false);
    }
  }

  if (loading) {
    return (
      <div className="flex flex-col gap-6">
        <h1 className="flex items-center gap-2">
          <UsersIcon className="h-7 w-7" />
          Люди
        </h1>
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {Array.from({ length: 6 }).map((_, i) => (
            <Skeleton key={i} className="h-28 rounded-xl" />
          ))}
        </div>
      </div>
    );
  }

  const groups = new Map<string, DirectoryUser[]>();
  for (const u of filtered) {
    const arr = groups.get(u.org_label) ?? [];
    arr.push(u);
    groups.set(u.org_label, arr);
  }
  let cardIdx = 0;

  return (
    <div className="flex flex-col gap-6">
      <h1 className="flex items-center gap-2">
        <UsersIcon className="h-7 w-7" />
        Люди
      </h1>

      <div className="flex flex-col sm:flex-row items-start sm:items-center gap-3">
        <div className="relative flex-1 max-w-sm">
          <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground pointer-events-none" />
          <Input
            placeholder="Пошук за позивним, ім'ям, підрозділом…"
            value={search}
            onChange={(e) => {
              setSearch(e.target.value);
              if (e.target.value) setSearchParams({ search: e.target.value }, { replace: true });
              else setSearchParams({}, { replace: true });
            }}
            className="pl-9"
          />
        </div>
        <Badge variant="secondary">{filtered.length}</Badge>
        {isAdmin && (
          <label className="flex items-center gap-2 text-sm cursor-pointer ml-auto">
            <Checkbox
              checked={showInactive}
              onCheckedChange={(v) => setShowInactive(v as boolean)}
            />
            Показати заблокованих
          </label>
        )}
      </div>

      {filtered.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
          <UsersIcon className="h-10 w-10 mb-2 opacity-30" />
          <p className="text-sm">Нікого не знайдено</p>
        </div>
      ) : (
        <div className="flex flex-col gap-6">
          {Array.from(groups.entries()).map(([org, members]) => (
            <div key={org}>
              <div className="flex items-center gap-2 mb-3">
                <div className="h-px flex-1" style={{ background: "var(--border)" }} />
                <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground flex items-center gap-1.5">
                  <Shield className="h-3 w-3" />
                  {org}
                  <Badge variant="secondary" className="text-[10px] px-1.5 py-0 ml-0.5">
                    {members.length}
                  </Badge>
                </span>
                <div className="h-px flex-1" style={{ background: "var(--border)" }} />
              </div>
              <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                {members.map((u) => (
                  <UserCard
                    key={u.user_id}
                    user={u}
                    index={cardIdx++}
                    isAdmin={isAdmin}
                    isSelf={u.user_id === authUser?.user_id}
                    onToggleActive={handleToggleActive}
                    onResetPassword={(id) => { setResetDialogUser(id); setNewPassword(""); }}
                    onMessage={async (id) => {
                      try {
                        const res = await api.post<{ room_id: number }>(`/chat/dm/${id}`, {});
                        navigate(`/chat?room=${res.room_id}`);
                      } catch {
                        toast.error("Не вдалось відкрити діалог");
                      }
                    }}
                  />
                ))}
              </div>
            </div>
          ))}
        </div>
      )}

      <Dialog open={resetDialogUser !== null} onOpenChange={(open) => { if (!open) setResetDialogUser(null); }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Скинути пароль</DialogTitle>
          </DialogHeader>
          <div className="flex flex-col gap-4 pt-2">
            <div className="flex flex-col gap-1.5">
              <Label>Новий тимчасовий пароль</Label>
              <Input
                type="password"
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
                placeholder="Мін. 6 символів"
              />
            </div>
            <Button onClick={handleResetPassword} disabled={!newPassword || newPassword.length < 6 || resetting}>
              {resetting ? <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" /> : <KeyRound className="mr-1.5 h-3.5 w-3.5" />}
              Скинути
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
