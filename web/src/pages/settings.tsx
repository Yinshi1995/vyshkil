import { useEffect, useState, useCallback, useRef, type PointerEvent as RPointerEvent } from "react";
import Cropper from "react-easy-crop";
import type { Area } from "react-easy-crop";
import { Link } from "react-router-dom";
import { useAuth } from "@/context/auth";
import { useTheme } from "@/context/theme";
import { useConfirm } from "@/components/confirm-dialog";
import { api } from "@/api/client";
import type { AdminUserRow, AccountInfo, WhatsAppStatus, WaDestination, WaSubscription, NotificationType, DictionariesOverview, DictionaryEntry } from "@/api/types";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Separator } from "@/components/ui/separator";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { toast } from "sonner";
import {
  Command,
  CommandEmpty,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { OrgSearchResult } from "@/api/types";
import {
  Settings,
  UserPlus,
  KeyRound,
  ToggleLeft,
  ToggleRight,
  User,
  Users as UsersIcon,
  Moon,
  Sun,
  MessageCircle,
  Shield,
  Smartphone,
  Wifi,
  WifiOff,
  Bell,
  Palette,
  Lock,
  LogOut,
  Send,
  Loader2,
  QrCode,
  Plus,
  Trash2,
  Phone,
  ChevronDown,
  ChevronRight,
  BookOpen,
  Pencil,
  Download,
  FileSpreadsheet,
  MapPin,
  ChevronsUpDown,
  Check,
  Camera,
  X,
  ChevronUp,
  Eye,
  EyeOff,
} from "lucide-react";

// ---------------------------------------------------------------------------
// Account
// ---------------------------------------------------------------------------

function cropImage(imageSrc: string, crop: Area): Promise<Blob> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => {
      const canvas = document.createElement("canvas");
      const size = 512;
      canvas.width = size;
      canvas.height = size;
      const ctx = canvas.getContext("2d");
      if (!ctx) return reject(new Error("no canvas ctx"));
      ctx.drawImage(img, crop.x, crop.y, crop.width, crop.height, 0, 0, size, size);
      canvas.toBlob(
        (blob) => (blob ? resolve(blob) : reject(new Error("no blob"))),
        "image/jpeg",
        0.9,
      );
    };
    img.onerror = reject;
    img.src = imageSrc;
  });
}

function AvatarCropDialog({
  src,
  open,
  onClose,
  onCropped,
}: {
  src: string;
  open: boolean;
  onClose: () => void;
  onCropped: (blob: Blob) => void;
}) {
  const [crop, setCrop] = useState({ x: 0, y: 0 });
  const [zoom, setZoom] = useState(1);
  const [croppedArea, setCroppedArea] = useState<Area | null>(null);

  const onCropComplete = useCallback((_: Area, areaPixels: Area) => {
    setCroppedArea(areaPixels);
  }, []);

  async function handleSave() {
    if (!croppedArea) return;
    try {
      const blob = await cropImage(src, croppedArea);
      onCropped(blob);
    } catch {
      toast.error("Помилка обрізки");
    }
  }

  if (!open) return null;

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent className="sm:max-w-[440px] p-0 overflow-hidden" style={{ background: "var(--card)" }}>
        <DialogHeader className="px-4 pt-4 pb-0">
          <DialogTitle className="text-sm font-semibold" style={{ fontFamily: "var(--font-heading)" }}>
            Обрізати фото
          </DialogTitle>
        </DialogHeader>
        <div className="relative w-full" style={{ height: 320, background: "#000" }}>
          <Cropper
            image={src}
            crop={crop}
            zoom={zoom}
            aspect={1}
            cropShape="round"
            showGrid={false}
            onCropChange={setCrop}
            onZoomChange={setZoom}
            onCropComplete={onCropComplete}
          />
        </div>
        <div className="px-4 py-3 space-y-3">
          <div className="flex items-center gap-3">
            <span className="text-[11px] text-muted-foreground shrink-0">Масштаб</span>
            <input
              type="range"
              min={1}
              max={3}
              step={0.05}
              value={zoom}
              onChange={(e) => setZoom(Number(e.target.value))}
              className="flex-1 accent-[var(--primary)]"
              style={{ height: 4 }}
            />
          </div>
          <div className="flex justify-end gap-2">
            <Button variant="outline" size="sm" onClick={onClose}>
              Скасувати
            </Button>
            <Button size="sm" onClick={handleSave}>
              Зберегти
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

function AvatarUpload({ url, onUploaded }: { url: string | null; onUploaded: (url: string | null) => void }) {
  const fileRef = useRef<HTMLInputElement>(null);
  const [uploading, setUploading] = useState(false);
  const [cropSrc, setCropSrc] = useState<string | null>(null);

  function handleFileSelect(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => setCropSrc(reader.result as string);
    reader.readAsDataURL(file);
    if (fileRef.current) fileRef.current.value = "";
  }

  async function handleCropped(blob: Blob) {
    setCropSrc(null);
    setUploading(true);
    try {
      const fd = new FormData();
      fd.append("avatar", blob, "avatar.jpg");
      const res = await fetch("/api/auth/avatar", { method: "POST", body: fd, credentials: "include" });
      if (!res.ok) throw new Error();
      const data = await res.json();
      onUploaded(data.avatar_url);
      toast.success("Аватар оновлено");
    } catch {
      toast.error("Помилка завантаження");
    } finally {
      setUploading(false);
    }
  }

  async function handleRemove() {
    try {
      await fetch("/api/auth/avatar", { method: "DELETE", credentials: "include" });
      onUploaded(null);
      toast.success("Аватар видалено");
    } catch {
      toast.error("Помилка");
    }
  }

  return (
    <div className="relative group">
      <div
        className="h-24 w-24 rounded-full border-2 border-primary/30 overflow-hidden flex items-center justify-center transition-all duration-300 group-hover:border-primary/60 group-hover:shadow-lg group-hover:shadow-primary/10"
        style={{ background: "var(--muted)" }}
      >
        {url ? (
          <img src={url} alt="avatar" className="h-full w-full object-cover" />
        ) : (
          <User className="h-10 w-10 text-muted-foreground" />
        )}
      </div>
      <button
        type="button"
        onClick={() => fileRef.current?.click()}
        className="absolute bottom-0 right-0 h-8 w-8 rounded-full flex items-center justify-center border-2 transition-all duration-200 hover:scale-110"
        style={{ background: "var(--primary)", borderColor: "var(--background)", color: "var(--primary-foreground)" }}
        disabled={uploading}
      >
        {uploading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Camera className="h-3.5 w-3.5" />}
      </button>
      {url && (
        <button
          type="button"
          onClick={handleRemove}
          className="absolute top-0 right-0 h-5 w-5 rounded-full flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity duration-200"
          style={{ background: "var(--destructive)", color: "var(--destructive-foreground)" }}
        >
          <X className="h-3 w-3" />
        </button>
      )}
      <input ref={fileRef} type="file" accept="image/*" className="hidden" onChange={handleFileSelect} />
      {cropSrc && (
        <AvatarCropDialog
          src={cropSrc}
          open={!!cropSrc}
          onClose={() => setCropSrc(null)}
          onCropped={handleCropped}
        />
      )}
    </div>
  );
}

const ROLE_UA: Record<string, string> = {
  admin: "Адмін",
  org_editor: "Редактор",
  viewer: "Переглядач",
};

function AccountSection() {
  const [info, setInfo] = useState<AccountInfo | null>(null);
  const { actor, logout, refresh: refreshUser } = useAuth();
  const [firstName, setFirstName] = useState("");
  const [lastName, setLastName] = useState("");
  const [rank, setRank] = useState("");
  const [phone, setPhone] = useState("");
  const [callsign, setCallsign] = useState("");
  const [deltaNick, setDeltaNick] = useState("");
  const [avatarUrl, setAvatarUrl] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [dirty, setDirty] = useState(false);
  const [rolesExpanded, setRolesExpanded] = useState(false);

  useEffect(() => {
    api.get<AccountInfo>("/auth/account").then((data) => {
      setInfo(data);
      setFirstName(data.first_name ?? "");
      setLastName(data.last_name ?? "");
      setRank(data.rank ?? "");
      setPhone(data.phone ?? "");
      setCallsign(data.callsign ?? "");
      setDeltaNick(data.delta_nick ?? "");
      setAvatarUrl(data.avatar_url);
    }).catch(() => {});
  }, []);

  async function handleSaveProfile() {
    setSaving(true);
    try {
      await api.put("/auth/account", {
        first_name: firstName.trim() || null,
        last_name: lastName.trim() || null,
        rank: rank.trim() || null,
        phone: phone.trim() || null,
        callsign: callsign.trim() || null,
        delta_nick: deltaNick.trim() || null,
      });
      toast.success("Профіль оновлено");
      setDirty(false);
      refreshUser();
    } catch {
      toast.error("Помилка збереження");
    } finally {
      setSaving(false);
    }
  }

  const d = (setter: (v: string) => void) => (e: React.ChangeEvent<HTMLInputElement>) => {
    setter(e.target.value);
    setDirty(true);
  };

  if (!info) return <Skeleton className="h-64 w-full rounded-lg" />;

  const ROLES_PREVIEW = 5;
  const hasMore = info.roles.length > ROLES_PREVIEW;
  const visibleRoles = rolesExpanded ? info.roles : info.roles.slice(0, ROLES_PREVIEW);

  return (
    <div className="flex flex-col gap-4">
      {/* Profile card with avatar */}
      <Card className="overflow-hidden">
        <div className="h-16" style={{ background: "linear-gradient(135deg, var(--primary) 0%, color-mix(in oklch, var(--primary) 60%, transparent) 100%)" }} />
        <CardContent className="relative pt-0 -mt-10">
          <div className="flex flex-col sm:flex-row gap-4 sm:items-end">
            <AvatarUpload url={avatarUrl} onUploaded={(url) => { setAvatarUrl(url); refreshUser(); }} />
            <div className="flex-1 flex flex-col gap-0.5 pb-1">
              <h3 className="text-lg font-bold leading-tight">
                {callsign || firstName || info.login}
              </h3>
              {(firstName || lastName) && (
                <p className="text-sm text-muted-foreground">
                  {[rank, firstName, lastName].filter(Boolean).join(" ")}
                </p>
              )}
              {actor && (
                <p className="text-xs text-muted-foreground mt-0.5">
                  {actor.role} — {actor.org_label}
                </p>
              )}
            </div>
          </div>

          <Separator className="my-4" />

          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="callsign">Позивний / нік</Label>
              <Input id="callsign" placeholder="Ваш позивний" value={callsign} onChange={d(setCallsign)} />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="firstName">Ім'я</Label>
              <Input id="firstName" placeholder="Ім'я" value={firstName} onChange={d(setFirstName)} />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="lastName">Прізвище</Label>
              <Input id="lastName" placeholder="Прізвище" value={lastName} onChange={d(setLastName)} />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="rank">Звання</Label>
              <Input id="rank" placeholder="напр. капітан" value={rank} onChange={d(setRank)} />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="phone">Телефон</Label>
              <Input id="phone" placeholder="+380..." value={phone} onChange={d(setPhone)} />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="deltaNick">IXD (Delta)</Label>
              <Input id="deltaNick" placeholder="нік в IXD" value={deltaNick} onChange={d(setDeltaNick)} />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label>Логін</Label>
              <Input value={info.login} disabled className="opacity-60" />
            </div>
          </div>

          {dirty && (
            <div className="mt-4">
              <Button size="sm" onClick={handleSaveProfile} disabled={saving}>
                {saving ? <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" /> : <Check className="mr-1.5 h-3.5 w-3.5" />}
                Зберегти
              </Button>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Roles — collapsible */}
      <Card>
        <CardContent className="pt-4 pb-3">
          <button
            type="button"
            className="flex items-center gap-2 text-sm font-medium w-full text-left"
            onClick={() => setRolesExpanded(!rolesExpanded)}
          >
            <Shield className="h-4 w-4 text-primary" />
            Ролі
            <Badge variant="secondary" className="ml-1 text-xs">{info.roles.length}</Badge>
            <ChevronUp className={`ml-auto h-4 w-4 text-muted-foreground transition-transform duration-200 ${rolesExpanded ? "" : "rotate-180"}`} />
          </button>
          <div className="flex flex-wrap gap-1.5 mt-2 overflow-hidden transition-all duration-300" style={{ maxHeight: rolesExpanded ? "500px" : "32px" }}>
            {visibleRoles.map((r, i) => (
              <Badge key={i} variant="outline" className="text-xs animate-in fade-in duration-200" style={{ animationDelay: `${i * 30}ms` }}>
                {r}
              </Badge>
            ))}
            {!rolesExpanded && hasMore && (
              <Badge variant="secondary" className="text-xs cursor-pointer" onClick={() => setRolesExpanded(true)}>
                +{info.roles.length - ROLES_PREVIEW}
              </Badge>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Security */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="flex items-center gap-2 text-sm">
            <Lock className="h-4 w-4 text-primary" />
            Безпека
          </CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium">Змінити пароль</p>
              <p className="text-xs text-muted-foreground">Рекомендуємо міняти кожні 90 днів</p>
            </div>
            <Link to="/change-password">
              <Button variant="outline" size="sm">
                <KeyRound className="mr-1.5 h-3.5 w-3.5" />
                Змінити
              </Button>
            </Link>
          </div>
          <Separator />
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium">Вийти з системи</p>
              <p className="text-xs text-muted-foreground">Завершити поточну сесію</p>
            </div>
            <Button variant="destructive" size="sm" onClick={() => logout()}>
              <LogOut className="mr-1.5 h-3.5 w-3.5" />
              Вихід
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Appearance
// ---------------------------------------------------------------------------

function AppearanceSection() {
  const { theme, toggle } = useTheme();

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-sm">
          <Palette className="h-4 w-4 text-primary" />
          Вигляд
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            {theme === "dark" ? (
              <Moon className="h-5 w-5 text-primary" />
            ) : (
              <Sun className="h-5 w-5 text-primary" />
            )}
            <div>
              <p className="text-sm font-medium">
                {theme === "dark" ? "Нічна тема" : "Денна тема"}
              </p>
              <p className="text-xs text-muted-foreground">
                {theme === "dark"
                  ? "Темний фон — для роботи вночі"
                  : "Світлий фон — для денного використання"}
              </p>
            </div>
          </div>
          <Button variant="outline" size="sm" onClick={toggle}>
            {theme === "dark" ? (
              <Sun className="mr-1.5 h-3.5 w-3.5" />
            ) : (
              <Moon className="mr-1.5 h-3.5 w-3.5" />
            )}
            {theme === "dark" ? "Денна" : "Нічна"}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// WhatsApp (admin only)
// ---------------------------------------------------------------------------

function WhatsAppSection() {
  const { actor } = useAuth();
  const [status, setStatus] = useState<WhatsAppStatus | null>(null);
  const [phone, setPhone] = useState("");
  const [pairing, setPairing] = useState(false);
  const [testing, setTesting] = useState(false);
  const [loggingOut, setLoggingOut] = useState(false);
  const eventSourceRef = useRef<EventSource | null>(null);

  useEffect(() => {
    if (!actor) return;
    const es = new EventSource(
      `/api/admin/whatsapp/events?org_id=${actor.org_id}&role=${actor.role}`,
    );
    eventSourceRef.current = es;
    es.onmessage = (e) => {
      try {
        setStatus(JSON.parse(e.data));
      } catch {}
    };
    es.onerror = () => {
      setStatus(null);
    };
    return () => es.close();
  }, [actor]);

  async function handlePair() {
    if (!phone) return;
    setPairing(true);
    try {
      const res = await api.post<{ pairing_code: string }>("/admin/whatsapp/pair", { phone });
      toast.success(`Pairing code: ${res.pairing_code}`);
    } catch {
      toast.error("Не вдалося запросити pairing-код");
    } finally {
      setPairing(false);
    }
  }

  async function handleLogout() {
    setLoggingOut(true);
    try {
      await api.post("/admin/whatsapp/logout");
      toast.success("WhatsApp від'єднано");
    } catch {
      toast.error("Помилка");
    } finally {
      setLoggingOut(false);
    }
  }

  async function handleTest() {
    if (!actor) return;
    setTesting(true);
    try {
      await api.post("/admin/whatsapp/test", { org_id: actor.org_id });
      toast.success("Тестове повідомлення надіслано");
    } catch {
      toast.error("Помилка надсилання");
    } finally {
      setTesting(false);
    }
  }

  const connected = status?.state === "connected";
  const notRunning = !status || status.state === "not_running";
  const starting = status?.state === "starting";
  const serviceUnavailable = notRunning || starting;
  const hasQr = status?.qr_svg;

  return (
    <div className="flex flex-col gap-4">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-sm">
            <MessageCircle className="h-4 w-4 text-primary" />
            Статус підключення
            {connected ? (
              <Badge variant="default" className="ml-auto">
                <Wifi className="mr-1 h-3 w-3" />
                Підключено
              </Badge>
            ) : status?.state === "not_running" ? (
              <Badge variant="secondary" className="ml-auto">
                <WifiOff className="mr-1 h-3 w-3" />
                Сервіс не запущено
              </Badge>
            ) : status?.state === "starting" ? (
              <Badge variant="outline" className="ml-auto">
                <Loader2 className="mr-1 h-3 w-3 animate-spin" />
                Запускається…
              </Badge>
            ) : status?.state === "pairing" ? (
              <Badge variant="outline" className="ml-auto border-primary/50 text-primary">
                <Loader2 className="mr-1 h-3 w-3 animate-spin" />
                Очікує прив'язки
              </Badge>
            ) : status?.state === "needs_pairing" ? (
              <Badge variant="secondary" className="ml-auto">
                <WifiOff className="mr-1 h-3 w-3" />
                Потребує прив'язки
              </Badge>
            ) : (
              <Badge variant="destructive" className="ml-auto">
                <WifiOff className="mr-1 h-3 w-3" />
                {status?.state ?? "Невідомо"}
              </Badge>
            )}
          </CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          {connected && status?.phone_masked && (
            <div className="flex items-center gap-3">
              <Smartphone className="h-5 w-5 text-primary" />
              <div>
                <p className="text-sm font-medium">{status.phone_masked}</p>
                <p className="text-xs text-muted-foreground">
                  Оновлено: {status.updated_at}
                </p>
              </div>
            </div>
          )}

          {hasQr && (
            <div className="flex flex-col items-center gap-3 py-2">
              <div className="eyebrow">Відскануйте QR-код у WhatsApp</div>
              <div
                className="rounded-lg border border-border bg-white p-3"
                dangerouslySetInnerHTML={{ __html: status!.qr_svg! }}
              />
            </div>
          )}

          {status?.pairing_code && (
            <div className="flex items-center gap-2 rounded-lg border border-primary/30 bg-primary/5 p-3">
              <QrCode className="h-5 w-5 text-primary" />
              <div>
                <p className="text-xs text-muted-foreground">Pairing-код</p>
                <p className="font-mono text-lg font-bold text-primary">
                  {status.pairing_code}
                </p>
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-sm">
            <Shield className="h-4 w-4 text-primary" />
            Керування
          </CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          {serviceUnavailable && (
            <p className="text-sm text-muted-foreground rounded-lg border border-border bg-muted/50 px-3 py-2">
              {starting ? "Сервіс запускається — зачекайте…" : "Сервіс нотифікацій не запущено — керування недоступне."}
            </p>
          )}
          <div className="flex flex-col gap-2">
            <Label>Номер телефону (для pairing-коду)</Label>
            <div className="flex gap-2">
              <Input
                value={phone}
                onChange={(e) => setPhone(e.target.value)}
                placeholder="380XXXXXXXXX"
                className="flex-1"
              />
              <Button onClick={handlePair} disabled={pairing || !phone || serviceUnavailable || connected} size="sm">
                {pairing ? <Loader2 className="h-4 w-4 animate-spin" /> : "Запросити код"}
              </Button>
            </div>
          </div>
          <Separator />
          <div className="flex flex-wrap gap-2">
            <Button variant="outline" size="sm" onClick={handleTest} disabled={testing || !connected}>
              {testing ? (
                <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
              ) : (
                <Send className="mr-1.5 h-3.5 w-3.5" />
              )}
              Тестове повідомлення
            </Button>
            <Button variant="destructive" size="sm" onClick={handleLogout} disabled={loggingOut || serviceUnavailable}>
              {loggingOut ? (
                <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
              ) : (
                <LogOut className="mr-1.5 h-3.5 w-3.5" />
              )}
              Від'єднати WhatsApp
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Notifications
// ---------------------------------------------------------------------------

function NotificationsSection() {
  const { actor } = useAuth();
  const isAdmin = actor?.role === "admin";
  const [destinations, setDestinations] = useState<WaDestination[]>([]);
  const [types, setTypes] = useState<NotificationType[]>([]);
  const [newPhone, setNewPhone] = useState("");
  const [newKind, setNewKind] = useState<"personal" | "bot" | "group">("personal");
  const [adding, setAdding] = useState(false);
  const [expandedDest, setExpandedDest] = useState<number | null>(null);
  const [subs, setSubs] = useState<Record<number, WaSubscription[]>>({});
  const [loading, setLoading] = useState(true);
  const [newGroupName, setNewGroupName] = useState("");

  const loadDestinations = useCallback(async () => {
    try {
      const [dests, ntTypes] = await Promise.all([
        api.get<WaDestination[]>("/whatsapp/destinations"),
        api.get<NotificationType[]>("/whatsapp/notification-types"),
      ]);
      setDestinations(dests);
      setTypes(ntTypes);
    } catch {}
    setLoading(false);
  }, []);

  useEffect(() => { loadDestinations(); }, [loadDestinations]);

  async function handleAdd() {
    if (newKind === "group") {
      if (!newGroupName.trim()) return;
      setAdding(true);
      try {
        await api.post("/whatsapp/destinations", {
          kind: "group",
          phone_masked: newGroupName.trim(),
        });
        setNewGroupName("");
        toast.success("Групу додано");
        await loadDestinations();
      } catch {
        toast.error("Не вдалося додати групу");
      } finally {
        setAdding(false);
      }
    } else {
      if (!newPhone.trim()) return;
      setAdding(true);
      try {
        await api.post("/whatsapp/destinations", { kind: newKind, phone_masked: newPhone.trim() });
        setNewPhone("");
        toast.success("Контакт додано");
        await loadDestinations();
      } catch {
        toast.error("Не вдалося додати контакт");
      } finally {
        setAdding(false);
      }
    }
  }

  async function handleDelete(id: number) {
    try {
      await api.delete(`/whatsapp/destinations/${id}`);
      toast.success("Контакт видалено");
      setDestinations(prev => prev.filter(d => d.id !== id));
    } catch {
      toast.error("Помилка видалення");
    }
  }

  async function handleToggle(id: number, active: boolean) {
    try {
      await api.post(`/whatsapp/destinations/${id}/toggle`, { is_active: active });
      setDestinations(prev => prev.map(d => d.id === id ? { ...d, is_active: active } : d));
    } catch {
      toast.error("Помилка");
    }
  }

  async function loadSubs(destId: number) {
    try {
      const rows = await api.get<WaSubscription[]>(`/whatsapp/destinations/${destId}/subscriptions`);
      setSubs(prev => ({ ...prev, [destId]: rows }));
    } catch {}
  }

  function toggleExpand(destId: number) {
    if (expandedDest === destId) {
      setExpandedDest(null);
    } else {
      setExpandedDest(destId);
      if (!subs[destId]) loadSubs(destId);
    }
  }

  async function handleSubToggle(destId: number, typeCode: string, active: boolean) {
    try {
      await api.post(`/whatsapp/destinations/${destId}/subscriptions`, {
        notification_type_code: typeCode,
        is_active: active,
      });
      setSubs(prev => {
        const current = prev[destId] || [];
        const existing = current.find(s => s.notification_type_code === typeCode);
        if (existing) {
          return { ...prev, [destId]: current.map(s => s.notification_type_code === typeCode ? { ...s, is_active: active } : s) };
        }
        const typeName = types.find(t => t.code === typeCode)?.name ?? typeCode;
        return { ...prev, [destId]: [...current, { id: 0, destination_id: destId, notification_type_code: typeCode, type_name: typeName, is_active: active }] };
      });
    } catch {
      toast.error("Помилка підписки");
    }
  }

  if (loading) return <Skeleton className="h-40 w-full" />;

  return (
    <div className="flex flex-col gap-4">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-sm">
            <Phone className="h-4 w-4 text-primary" />
            Контакти для сповіщень
          </CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <p className="text-sm text-muted-foreground">
            Додайте номери WhatsApp або групи для отримання сповіщень. Натисніть на контакт, щоб налаштувати підписки.
          </p>

          <div className="flex gap-2 items-end">
            <div>
              <Label className="text-xs">Тип</Label>
              <Select value={newKind} onValueChange={v => setNewKind(v as "personal" | "bot" | "group")}>
                <SelectTrigger className="mt-1 w-[150px]">
                  <SelectValue>
                    {newKind === "personal" ? "Особистий" : newKind === "bot" ? "Бот" : "Група"}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="personal">Особистий</SelectItem>
                  <SelectItem value="bot">Бот</SelectItem>
                  {isAdmin && <SelectItem value="group">Група</SelectItem>}
                </SelectContent>
              </Select>
            </div>
            <div className="flex-1">
              {newKind === "group" ? (
                <>
                  <Label className="text-xs">Назва групи WhatsApp</Label>
                  <Input
                    value={newGroupName}
                    onChange={e => setNewGroupName(e.target.value)}
                    placeholder="Назва групи точно як у WhatsApp"
                    className="mt-1"
                  />
                </>
              ) : (
                <>
                  <Label className="text-xs">Номер телефону</Label>
                  <Input
                    value={newPhone}
                    onChange={e => setNewPhone(e.target.value)}
                    placeholder="380XXXXXXXXX"
                    className="mt-1"
                  />
                </>
              )}
            </div>
            <Button
              size="sm"
              onClick={handleAdd}
              disabled={adding || (newKind === "group" ? !newGroupName.trim() : !newPhone.trim())}
            >
              {adding ? <Loader2 className="h-4 w-4 animate-spin" /> : <Plus className="h-4 w-4" />}
            </Button>
          </div>
          {newKind === "group" && (
            <p className="text-xs text-muted-foreground">
              Бот повинен бути учасником цієї групи. Введіть назву точно як у WhatsApp.
            </p>
          )}

          {destinations.length === 0 ? (
            <p className="text-sm text-muted-foreground text-center py-4">Немає контактів</p>
          ) : (
            <div className="flex flex-col gap-1">
              {destinations.map(dest => (
                <div key={dest.id} className="rounded-lg border border-border">
                  <div
                    className="flex items-center gap-3 px-3 py-2 cursor-pointer hover:bg-muted/50"
                    onClick={() => toggleExpand(dest.id)}
                  >
                    {expandedDest === dest.id ? (
                      <ChevronDown className="h-4 w-4 shrink-0 text-muted-foreground" />
                    ) : (
                      <ChevronRight className="h-4 w-4 shrink-0 text-muted-foreground" />
                    )}
                    {dest.kind === "group" ? (
                      <UsersIcon className="h-4 w-4 shrink-0 text-emerald-500" />
                    ) : (
                      <Phone className="h-4 w-4 shrink-0 text-primary" />
                    )}
                    <span className="text-sm font-medium flex-1">{dest.phone_masked}</span>
                    <Badge
                      variant={dest.kind === "group" ? "default" : dest.kind === "bot" ? "secondary" : "outline"}
                      className={`text-xs ${dest.kind === "group" ? "bg-emerald-500/15 text-emerald-700 border-emerald-500/30" : ""}`}
                    >
                      {dest.kind === "group" ? "Група" : dest.kind === "bot" ? "Бот" : "Особистий"}
                    </Badge>
                    <button
                      onClick={e => { e.stopPropagation(); handleToggle(dest.id, !dest.is_active); }}
                      className="text-muted-foreground hover:text-foreground"
                      title={dest.is_active ? "Вимкнути" : "Увімкнути"}
                    >
                      {dest.is_active ? (
                        <ToggleRight className="h-5 w-5 text-primary" />
                      ) : (
                        <ToggleLeft className="h-5 w-5" />
                      )}
                    </button>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-6 w-6 text-muted-foreground hover:text-destructive"
                      onClick={e => { e.stopPropagation(); handleDelete(dest.id); }}
                      title="Видалити"
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>

                  {expandedDest === dest.id && (
                    <div className="border-t border-border px-3 py-2 bg-muted/30">
                      <p className="text-xs text-muted-foreground mb-2">Підписки на сповіщення:</p>
                      <div className="flex flex-col gap-1">
                        {types.filter(t => t.scope === "org").map(nt => {
                          const sub = subs[dest.id]?.find(s => s.notification_type_code === nt.code);
                          const active = sub?.is_active ?? false;
                          return (
                            <div key={nt.code} className="flex items-center justify-between py-1">
                              <span className="text-sm">{nt.name}</span>
                              <button
                                onClick={() => handleSubToggle(dest.id, nt.code, !active)}
                                className="text-muted-foreground hover:text-foreground"
                              >
                                {active ? (
                                  <ToggleRight className="h-5 w-5 text-primary" />
                                ) : (
                                  <ToggleLeft className="h-5 w-5" />
                                )}
                              </button>
                            </div>
                          );
                        })}
                      </div>
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

// ---------------------------------------------------------------------------
// User Management (admin only)
// ---------------------------------------------------------------------------

function CreateUserDialog({ onCreated }: { onCreated: () => void }) {
  const [open, setOpen] = useState(false);
  const [login, setLogin] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [tempPassword, setTempPassword] = useState("");

  async function handleCreate() {
    try {
      await api.post("/admin/users", {
        login,
        display_name: displayName || null,
        temp_password: tempPassword,
      });
      toast.success("Обліковий запис створено");
      setOpen(false);
      setLogin("");
      setDisplayName("");
      setTempPassword("");
      onCreated();
    } catch {
      toast.error("Не вдалося створити обліковий запис");
    }
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger className="inline-flex items-center gap-1.5 rounded-lg bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground hover:bg-primary/80">
        <UserPlus className="h-3.5 w-3.5" />
        Додати
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Новий обліковий запис</DialogTitle>
        </DialogHeader>
        <div className="flex flex-col gap-4 pt-2">
          <div className="flex flex-col gap-1.5">
            <Label>Логін</Label>
            <Input value={login} onChange={(e) => setLogin(e.target.value)} />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label>Ім'я (необов'язково)</Label>
            <Input value={displayName} onChange={(e) => setDisplayName(e.target.value)} />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label>Тимчасовий пароль</Label>
            <Input
              type="password"
              value={tempPassword}
              onChange={(e) => setTempPassword(e.target.value)}
            />
          </div>
          <Button onClick={handleCreate} disabled={!login || !tempPassword}>
            Створити
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

function UserManagementSection() {
  const [users, setUsers] = useState<AdminUserRow[] | null>(null);
  const { prompt: promptDialog, dialog: confirmDialog } = useConfirm();

  const loadUsers = useCallback(() => {
    api.get<AdminUserRow[]>("/admin/users").then(setUsers).catch(() => setUsers([]));
  }, []);

  useEffect(() => {
    loadUsers();
  }, [loadUsers]);

  async function handleResetPassword(userId: number) {
    const pw = await promptDialog({
      title: "Скидання пароля",
      description: "Введіть новий тимчасовий пароль для користувача.",
      confirmLabel: "Скинути",
      input: { label: "Новий пароль", type: "password", placeholder: "Тимчасовий пароль" },
    });
    if (!pw) return;
    try {
      await api.post(`/admin/users/${userId}/reset-password`, { temp_password: pw });
      toast.success("Пароль скинуто");
      loadUsers();
    } catch {
      toast.error("Помилка скидання пароля");
    }
  }

  async function handleToggleActive(userId: number) {
    try {
      await api.post(`/admin/users/${userId}/toggle-active`);
      loadUsers();
    } catch {
      toast.error("Помилка зміни статусу");
    }
  }

  async function handleToggleOrgNames(userId: number) {
    try {
      await api.post(`/admin/users/${userId}/toggle-org-names`);
      loadUsers();
    } catch {
      toast.error("Помилка зміни видимості назв");
    }
  }

  if (!users) return <Skeleton className="h-48 w-full" />;

  return (
    <Card>
      {confirmDialog}
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle className="flex items-center gap-2 text-sm">
          <UsersIcon className="h-4 w-4 text-primary" />
          Користувачі
        </CardTitle>
        <CreateUserDialog onCreated={loadUsers} />
      </CardHeader>
      <CardContent>
        {users.length === 0 ? (
          <p className="py-8 text-center text-sm text-muted-foreground">Немає користувачів</p>
        ) : (
          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Логін</TableHead>
                  <TableHead>Ім'я</TableHead>
                  <TableHead>Ролі</TableHead>
                  <TableHead>Статус</TableHead>
                  <TableHead className="text-right">Дії</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {users.map((u) => (
                  <TableRow key={u.id}>
                    <TableCell className="font-medium">{u.login}</TableCell>
                    <TableCell>{u.display_name ?? "—"}</TableCell>
                    <TableCell>
                      <div className="flex flex-wrap gap-1">
                        {u.roles.map((r, i) => (
                          <Badge key={i} variant="outline" className="text-xs">
                            {r.role}@{r.org_label}
                          </Badge>
                        ))}
                      </div>
                    </TableCell>
                    <TableCell>
                      <Badge variant={u.is_active ? "default" : "destructive"}>
                        {u.is_active ? "Активний" : "Неактивний"}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="flex justify-end gap-1">
                        <Button
                          variant="ghost"
                          size="icon"
                          title="Скинути пароль"
                          onClick={() => handleResetPassword(u.id)}
                        >
                          <KeyRound className="h-4 w-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          title={u.can_see_org_names ? "Заборонити бачити назви частин" : "Дозволити бачити назви частин"}
                          onClick={() => handleToggleOrgNames(u.id)}
                        >
                          {u.can_see_org_names ? (
                            <Eye className="h-4 w-4 text-green-500" />
                          ) : (
                            <EyeOff className="h-4 w-4" />
                          )}
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          title={u.is_active ? "Деактивувати" : "Активувати"}
                          onClick={() => handleToggleActive(u.id)}
                        >
                          {u.is_active ? (
                            <ToggleRight className="h-4 w-4" />
                          ) : (
                            <ToggleLeft className="h-4 w-4" />
                          )}
                        </Button>
                      </div>
                    </TableCell>
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

// ---------------------------------------------------------------------------
// Dictionaries (admin only) — ВОС & ОВТ management
// ---------------------------------------------------------------------------

type DictTypeKey =
  | "vos"
  | "equipment"
  | "training_kind"
  | "training_direction"
  | "bzvp_program"
  | "position"
  | "course"
  | "attrition_reason";

interface DictConfig {
  key: DictTypeKey;
  overviewField: keyof DictionariesOverview;
  title: string;
  tabLabel: string;
  codeLabel: string;
  nameLabel: string;
  hasCode?: boolean;
  hasCategory?: boolean;
  hasRequiresNote?: boolean;
  codePlaceholder?: string;
  namePlaceholder?: string;
}

const DICT_CONFIGS: DictConfig[] = [
  {
    key: "vos", overviewField: "vos",
    title: "ВОС (Військово-облікові спеціальності)", tabLabel: "ВОС",
    codeLabel: "Код", nameLabel: "Назва", hasCode: true,
    codePlaceholder: "напр. 217", namePlaceholder: "напр. Оператор БПЛА",
  },
  {
    key: "equipment", overviewField: "equipment",
    title: "ОВТ (Озброєння та військова техніка)", tabLabel: "ОВТ",
    codeLabel: "Категорія", nameLabel: "Назва", hasCategory: true,
    namePlaceholder: "напр. Mavic 3",
  },
  {
    key: "training_kind", overviewField: "training_kinds",
    title: "Види підготовки", tabLabel: "Види",
    codeLabel: "Код", nameLabel: "Назва", hasCode: true,
    codePlaceholder: "напр. special", namePlaceholder: "напр. Фахова",
  },
  {
    key: "training_direction", overviewField: "training_directions",
    title: "Напрями підготовки", tabLabel: "Напрями",
    codeLabel: "Код", nameLabel: "Назва", hasCode: true,
    codePlaceholder: "напр. combat", namePlaceholder: "напр. Бойова підготовка",
  },
  {
    key: "bzvp_program", overviewField: "bzvp_programs",
    title: "Програми БЗВП", tabLabel: "БЗВП",
    codeLabel: "#", nameLabel: "Назва",
    namePlaceholder: "напр. Програма 1",
  },
  {
    key: "position", overviewField: "positions",
    title: "Посади", tabLabel: "Посади",
    codeLabel: "#", nameLabel: "Назва",
    namePlaceholder: "напр. Командир відділення",
  },
  {
    key: "course", overviewField: "courses",
    title: "Курси", tabLabel: "Курси",
    codeLabel: "#", nameLabel: "Назва",
    namePlaceholder: "напр. КІБР",
  },
  {
    key: "attrition_reason", overviewField: "attrition_reasons",
    title: "Причини вибуття", tabLabel: "Вибуття",
    codeLabel: "Примітка", nameLabel: "Назва", hasRequiresNote: true,
    namePlaceholder: "напр. Поранення",
  },
];

function DictTabContent({
  cfg,
  entries,
  onAdd,
  onEdit,
  onDelete,
}: {
  cfg: DictConfig;
  entries: DictionaryEntry[];
  onAdd: () => void;
  onEdit: (entry: DictionaryEntry) => void;
  onDelete: (id: number) => void;
}) {
  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-sm font-medium">{cfg.title}</h3>
          <p className="text-xs text-muted-foreground">{entries.length} записів</p>
        </div>
        <Button variant="outline" size="sm" onClick={onAdd}>
          <Plus className="mr-1.5 h-3.5 w-3.5" />
          Додати
        </Button>
      </div>
      {entries.length === 0 ? (
        <div className="flex flex-col items-center justify-center rounded-lg border border-dashed border-border py-12">
          <BookOpen className="mb-2 h-8 w-8 text-muted-foreground/50" />
          <p className="text-sm text-muted-foreground">Записів немає</p>
          <Button variant="link" size="sm" className="mt-1" onClick={onAdd}>
            Додати перший запис
          </Button>
        </div>
      ) : (
        <Card>
          <CardContent className="p-0">
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>{cfg.codeLabel}</TableHead>
                    <TableHead>{cfg.nameLabel}</TableHead>
                    <TableHead className="w-[80px] text-right">Дії</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {entries.map((e) => {
                    const parts = e.label.split(" — ");
                    const colCode = parts.length > 1 ? parts[0] : "";
                    const colName = parts.length > 1 ? parts.slice(1).join(" — ") : e.label;
                    return (
                      <TableRow key={e.id}>
                        <TableCell className="font-mono text-sm">{colCode || e.extra || "—"}</TableCell>
                        <TableCell className="text-sm">{colName}</TableCell>
                        <TableCell className="text-right">
                          <div className="flex justify-end gap-1">
                            <Button variant="ghost" size="icon" title="Редагувати" onClick={() => onEdit(e)}>
                              <Pencil className="h-3.5 w-3.5" />
                            </Button>
                            <Button variant="ghost" size="icon" title="Видалити" onClick={() => onDelete(e.id)}>
                              <Trash2 className="h-3.5 w-3.5 text-destructive" />
                            </Button>
                          </div>
                        </TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Training Sites CRUD (admin only)
// ---------------------------------------------------------------------------

function OrgCombobox({
  value,
  label,
  onChange,
}: {
  value: number | null;
  label: string;
  onChange: (orgId: number, orgLabel: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<OrgSearchResult[]>([]);

  useEffect(() => {
    const t = setTimeout(() => {
      api
        .get<OrgSearchResult[]>(`/orgs/search?q=${encodeURIComponent(query)}&scope=visible`)
        .then(setResults)
        .catch(() => {});
    }, 200);
    return () => clearTimeout(t);
  }, [query]);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger
        render={(props) => (
          <Button
            variant="outline"
            className="w-full justify-between font-normal"
            {...props}
          >
            <span className="truncate">{value ? label : "Оберіть підрозділ…"}</span>
            <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
          </Button>
        )}
      />
      <PopoverContent className="w-[--anchor-width] p-0" align="start">
        <Command shouldFilter={false}>
          <CommandInput
            placeholder="Пошук підрозділу…"
            value={query}
            onValueChange={setQuery}
          />
          <CommandList>
            <CommandEmpty>Не знайдено</CommandEmpty>
            {results.map((r) => (
              <CommandItem
                key={r.org_id}
                onSelect={() => {
                  onChange(r.org_id, r.label);
                  setOpen(false);
                }}
              >
                <Check
                  className={`mr-2 h-4 w-4 ${value === r.org_id ? "opacity-100" : "opacity-0"}`}
                />
                {r.label}
              </CommandItem>
            ))}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}

function TrainingSitesTab({
  entries,
  onRefresh,
}: {
  entries: DictionaryEntry[];
  onRefresh: () => void;
}) {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editEntry, setEditEntry] = useState<DictionaryEntry | null>(null);
  const [locality, setLocality] = useState("");
  const [orgId, setOrgId] = useState<number | null>(null);
  const [orgLabel, setOrgLabel] = useState("");
  const { confirm: confirmDel, dialog: confirmDelDialog } = useConfirm();

  function openAdd() {
    setEditEntry(null);
    setLocality("");
    setOrgId(null);
    setOrgLabel("");
    setDialogOpen(true);
  }

  function openEdit(entry: DictionaryEntry) {
    setEditEntry(entry);
    setLocality(entry.label);
    setOrgLabel(entry.extra ?? "");
    setOrgId(entry.extra_id ?? null);
    setDialogOpen(true);
  }

  async function handleSave() {
    if (!locality.trim()) return;
    try {
      if (editEntry) {
        const body: { org_id: number; locality: string } = {
          org_id: orgId ?? 0,
          locality: locality.trim(),
        };
        if (!orgId) {
          toast.error("Оберіть підрозділ");
          return;
        }
        await api.put(`/admin/dictionaries/training_site/${editEntry.id}`, body);
        toast.success("Оновлено");
      } else {
        if (!orgId) {
          toast.error("Оберіть підрозділ");
          return;
        }
        await api.post("/admin/dictionaries/training_site", {
          org_id: orgId,
          locality: locality.trim(),
        });
        toast.success("Додано");
      }
      setDialogOpen(false);
      onRefresh();
    } catch {
      toast.error("Помилка збереження");
    }
  }

  async function handleDelete(id: number) {
    const ok = await confirmDel({
      title: "Видалити місце підготовки?",
      description: "Це можливо лише якщо немає пов'язаних груп підготовки.",
      confirmLabel: "Видалити",
      variant: "destructive",
    });
    if (!ok) return;
    try {
      await api.delete(`/admin/dictionaries/training_site/${id}`);
      toast.success("Видалено");
      onRefresh();
    } catch {
      toast.error("Не вдалося видалити — можливо, є пов'язані групи підготовки");
    }
  }

  return (
    <div className="flex flex-col gap-3">
      {confirmDelDialog}
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-sm font-medium">Місця проведення підготовки</h3>
          <p className="text-xs text-muted-foreground">{entries.length} записів</p>
        </div>
        <Button variant="outline" size="sm" onClick={openAdd}>
          <Plus className="mr-1.5 h-3.5 w-3.5" />
          Додати
        </Button>
      </div>

      {entries.length === 0 ? (
        <div className="flex flex-col items-center justify-center rounded-lg border border-dashed border-border py-12">
          <MapPin className="mb-2 h-8 w-8 text-muted-foreground/50" />
          <p className="text-sm text-muted-foreground">Записів немає</p>
          <Button variant="link" size="sm" className="mt-1" onClick={openAdd}>
            Додати перше місце
          </Button>
        </div>
      ) : (
        <Card>
          <CardContent className="p-0">
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Місце</TableHead>
                    <TableHead>Підрозділ</TableHead>
                    <TableHead className="w-[80px] text-right">Дії</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {entries.map((ts) => (
                    <TableRow key={ts.id}>
                      <TableCell className="text-sm">{ts.label}</TableCell>
                      <TableCell className="text-sm text-muted-foreground">{ts.extra ?? "—"}</TableCell>
                      <TableCell className="text-right">
                        <div className="flex justify-end gap-1">
                          <Button variant="ghost" size="icon" title="Редагувати" onClick={() => openEdit(ts)}>
                            <Pencil className="h-3.5 w-3.5" />
                          </Button>
                          <Button variant="ghost" size="icon" title="Видалити" onClick={() => handleDelete(ts.id)}>
                            <Trash2 className="h-3.5 w-3.5 text-destructive" />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          </CardContent>
        </Card>
      )}

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{editEntry ? "Редагувати місце" : "Додати місце підготовки"}</DialogTitle>
          </DialogHeader>
          <div className="flex flex-col gap-4 pt-2">
            <div className="flex flex-col gap-1.5">
              <Label>Назва місця</Label>
              <Input
                placeholder="напр. ПП «Рівне»"
                value={locality}
                onChange={(e) => setLocality(e.target.value)}
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label>Підрозділ (НЦ / ПП)</Label>
              <OrgCombobox
                value={orgId}
                label={orgLabel}
                onChange={(id, label) => {
                  setOrgId(id);
                  setOrgLabel(label);
                }}
              />
            </div>
            <Button onClick={handleSave} disabled={!locality.trim() || !orgId}>
              {editEntry ? "Зберегти" : "Додати"}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function DictionariesSection() {
  const [overview, setOverview] = useState<DictionariesOverview | null>(null);
  const [editConfig, setEditConfig] = useState<DictConfig | null>(null);
  const [editEntry, setEditEntry] = useState<DictionaryEntry | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [code, setCode] = useState("");
  const [name, setName] = useState("");
  const [category, setCategory] = useState("");
  const [requiresNote, setRequiresNote] = useState(false);
  const [downloading, setDownloading] = useState(false);
  const { confirm: confirmDel, dialog: confirmDelDialog } = useConfirm();

  const load = useCallback(() => {
    api.get<DictionariesOverview>("/admin/dictionaries").then(setOverview).catch(() => {});
  }, []);

  useEffect(() => { load(); }, [load]);

  function openAdd(cfg: DictConfig) {
    setEditConfig(cfg);
    setEditEntry(null);
    setCode("");
    setName("");
    setCategory("");
    setRequiresNote(false);
    setDialogOpen(true);
  }

  function openEdit(cfg: DictConfig, entry: DictionaryEntry) {
    setEditConfig(cfg);
    setEditEntry(entry);
    if (cfg.key === "vos") {
      const parts = entry.label.split(" — ");
      setCode(parts[0] || "");
      setName(parts.length > 1 ? parts.slice(1).join(" — ") : "");
    } else if (cfg.hasCode) {
      setName(entry.label);
      setCode(entry.extra || "");
    } else if (cfg.hasCategory) {
      setName(entry.label);
      setCategory(entry.extra || "");
    } else if (cfg.hasRequiresNote) {
      setName(entry.label);
      setRequiresNote(entry.extra === "потребує примітки");
    } else {
      setName(entry.label);
    }
    setDialogOpen(true);
  }

  async function handleSave() {
    if (!editConfig) return;
    try {
      const cfg = editConfig;
      if (cfg.key === "vos") {
        if (editEntry) {
          await api.put(`/admin/dictionaries/vos/${editEntry.id}`, { code, title: name });
        } else {
          await api.post("/admin/dictionaries/vos", { code, title: name });
        }
      } else if (cfg.key === "equipment") {
        if (editEntry) {
          await api.put(`/admin/dictionaries/equipment/${editEntry.id}`, { name, category: category || null });
        } else {
          await api.post("/admin/dictionaries/equipment", { name, category: category || null });
        }
      } else {
        const body: Record<string, unknown> = { name };
        if (cfg.hasCode) body.code = code;
        if (cfg.hasRequiresNote) body.requires_note = requiresNote;
        if (editEntry) {
          await api.put(`/admin/dictionaries/${cfg.key}/${editEntry.id}`, body);
        } else {
          await api.post(`/admin/dictionaries/${cfg.key}`, body);
        }
      }
      setDialogOpen(false);
      toast.success(editEntry ? "Оновлено" : "Додано");
      load();
    } catch {
      toast.error("Помилка збереження");
    }
  }

  async function handleDelete(cfg: DictConfig, id: number) {
    const ok = await confirmDel({
      title: "Видалити запис?",
      description: `Запис із довідника "${cfg.label}" буде видалено.`,
      confirmLabel: "Видалити",
      variant: "destructive",
    });
    if (!ok) return;
    try {
      await api.delete(`/admin/dictionaries/${cfg.key}/${id}`);
      toast.success("Видалено");
      load();
    } catch {
      toast.error("Помилка видалення");
    }
  }

  async function downloadTemplate() {
    setDownloading(true);
    try {
      const resp = await fetch("/api/import/template", { credentials: "include" });
      if (!resp.ok) throw new Error();
      const blob = await resp.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "import-template.xlsx";
      a.click();
      URL.revokeObjectURL(url);
      toast.success("Шаблон завантажено");
    } catch {
      toast.error("Помилка завантаження шаблону");
    } finally {
      setDownloading(false);
    }
  }

  if (!overview) return <Skeleton className="h-48 w-full" />;

  const dialogTitle = editConfig
    ? `${editEntry ? "Редагувати" : "Додати"} — ${editConfig.title}`
    : "";

  return (
    <div className="flex flex-col gap-4">
      {confirmDelDialog}
      <div className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">
          Довідники системи — ВОС, ОВТ, види підготовки та інші класифікатори.
        </p>
        <Button variant="outline" size="sm" onClick={downloadTemplate} disabled={downloading}>
          {downloading ? (
            <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
          ) : (
            <FileSpreadsheet className="mr-1.5 h-3.5 w-3.5" />
          )}
          Шаблон .xlsx
        </Button>
      </div>

      <Tabs defaultValue="vos">
        <div className="overflow-x-auto -mx-4 px-4">
          <TabsList variant="line" className="w-full justify-start">
            {DICT_CONFIGS.map((cfg) => (
              <TabsTrigger key={cfg.key} value={cfg.key}>
                {cfg.tabLabel}
                <Badge variant="secondary" className="ml-1.5 h-5 min-w-5 px-1 text-[10px]">
                  {overview[cfg.overviewField].length}
                </Badge>
              </TabsTrigger>
            ))}
            <TabsTrigger value="training_sites">
              Місця
              <Badge variant="secondary" className="ml-1.5 h-5 min-w-5 px-1 text-[10px]">
                {overview.training_sites.length}
              </Badge>
            </TabsTrigger>
          </TabsList>
        </div>

        {DICT_CONFIGS.map((cfg) => (
          <TabsContent key={cfg.key} value={cfg.key} className="pt-3">
            <DictTabContent
              cfg={cfg}
              entries={overview[cfg.overviewField]}
              onAdd={() => openAdd(cfg)}
              onEdit={(e) => openEdit(cfg, e)}
              onDelete={(id) => handleDelete(cfg, id)}
            />
          </TabsContent>
        ))}

        <TabsContent value="training_sites" className="pt-3">
          <TrainingSitesTab
            entries={overview.training_sites}
            onRefresh={load}
          />
        </TabsContent>
      </Tabs>

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{dialogTitle}</DialogTitle>
          </DialogHeader>
          <div className="flex flex-col gap-4 pt-2">
            {editConfig?.key === "vos" ? (
              <>
                <div className="flex flex-col gap-1.5">
                  <Label>Код ВОС</Label>
                  <Input placeholder={editConfig.codePlaceholder} value={code} onChange={(e) => setCode(e.target.value)} />
                </div>
                <div className="flex flex-col gap-1.5">
                  <Label>Назва</Label>
                  <Input placeholder={editConfig.namePlaceholder} value={name} onChange={(e) => setName(e.target.value)} />
                </div>
              </>
            ) : editConfig?.hasCategory ? (
              <>
                <div className="flex flex-col gap-1.5">
                  <Label>Назва</Label>
                  <Input placeholder={editConfig.namePlaceholder} value={name} onChange={(e) => setName(e.target.value)} />
                </div>
                <div className="flex flex-col gap-1.5">
                  <Label>Категорія (необов'язково)</Label>
                  <Input placeholder="напр. БПЛА" value={category} onChange={(e) => setCategory(e.target.value)} />
                </div>
              </>
            ) : editConfig?.hasCode ? (
              <>
                <div className="flex flex-col gap-1.5">
                  <Label>Назва</Label>
                  <Input placeholder={editConfig.namePlaceholder} value={name} onChange={(e) => setName(e.target.value)} />
                </div>
                <div className="flex flex-col gap-1.5">
                  <Label>Код</Label>
                  <Input placeholder={editConfig.codePlaceholder} value={code} onChange={(e) => setCode(e.target.value)} />
                </div>
              </>
            ) : editConfig?.hasRequiresNote ? (
              <>
                <div className="flex flex-col gap-1.5">
                  <Label>Назва</Label>
                  <Input placeholder={editConfig.namePlaceholder} value={name} onChange={(e) => setName(e.target.value)} />
                </div>
                <label className="flex items-center gap-2 text-sm cursor-pointer">
                  <Checkbox
                    checked={requiresNote}
                    onCheckedChange={(checked) => setRequiresNote(checked as boolean)}
                  />
                  Потребує примітки при вибутті
                </label>
              </>
            ) : (
              <div className="flex flex-col gap-1.5">
                <Label>Назва</Label>
                <Input placeholder={editConfig?.namePlaceholder} value={name} onChange={(e) => setName(e.target.value)} />
              </div>
            )}
            <Button
              onClick={handleSave}
              disabled={editConfig?.key === "vos" ? !code || !name : !name}
            >
              {editEntry ? "Зберегти" : "Додати"}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export function SettingsPage() {
  const { isAdmin } = useAuth();

  return (
    <div className="flex flex-col gap-6">
      <h1 className="flex items-center gap-2">
        <Settings className="h-7 w-7" />
        Налаштування
      </h1>

      <Tabs defaultValue="account">
        <TabsList className="flex-wrap gap-1">
          <TabsTrigger value="account">
            <User className="sm:mr-1.5 h-3.5 w-3.5" />
            <span className="hidden sm:inline">Обліковий запис</span>
          </TabsTrigger>
          <TabsTrigger value="appearance">
            <Palette className="sm:mr-1.5 h-3.5 w-3.5" />
            <span className="hidden sm:inline">Вигляд</span>
          </TabsTrigger>
          <TabsTrigger value="notifications">
            <Bell className="sm:mr-1.5 h-3.5 w-3.5" />
            <span className="hidden sm:inline">Сповіщення</span>
          </TabsTrigger>
          {isAdmin && (
            <TabsTrigger value="whatsapp">
              <MessageCircle className="sm:mr-1.5 h-3.5 w-3.5" />
              <span className="hidden sm:inline">WhatsApp</span>
            </TabsTrigger>
          )}
          {isAdmin && (
            <TabsTrigger value="users">
              <UsersIcon className="sm:mr-1.5 h-3.5 w-3.5" />
              <span className="hidden sm:inline">Користувачі</span>
            </TabsTrigger>
          )}
          {isAdmin && (
            <TabsTrigger value="dictionaries">
              <BookOpen className="sm:mr-1.5 h-3.5 w-3.5" />
              <span className="hidden sm:inline">Довідники</span>
            </TabsTrigger>
          )}
        </TabsList>

        <TabsContent value="account" className="flex flex-col gap-4 pt-2">
          <AccountSection />
        </TabsContent>

        <TabsContent value="appearance" className="flex flex-col gap-4 pt-2">
          <AppearanceSection />
        </TabsContent>

        <TabsContent value="notifications" className="flex flex-col gap-4 pt-2">
          <NotificationsSection />
        </TabsContent>

        {isAdmin && (
          <TabsContent value="whatsapp" className="flex flex-col gap-4 pt-2">
            <WhatsAppSection />
          </TabsContent>
        )}

        {isAdmin && (
          <TabsContent value="users" className="flex flex-col gap-4 pt-2">
            <UserManagementSection />
          </TabsContent>
        )}

        {isAdmin && (
          <TabsContent value="dictionaries" className="flex flex-col gap-4 pt-2">
            <DictionariesSection />
          </TabsContent>
        )}
      </Tabs>
    </div>
  );
}
