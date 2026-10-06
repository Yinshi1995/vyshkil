import { useEffect, useState, useRef, useCallback, useMemo } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { useAuth } from "@/context/auth";
import { api } from "@/api/client";
import type { ChatRoom, ChatMessage } from "@/api/types";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { toast } from "sonner";
import { useContextMenu, ContextMenuPortal, type ContextMenuEntry } from "@/components/context-menu";
import {
  MessageCircle,
  Send,
  Mic,
  Square,
  Paperclip,
  X,
  Play,
  Pause,
  Reply,
  Users,
  ChevronLeft,
  ChevronRight,
  Pencil,
  Trash2,
  Check,
  Megaphone,
  Building2,
  MessageSquare,
  Copy,
  type LucideIcon,
} from "lucide-react";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ROLE_COLORS: Record<string, string> = {
  admin: "#c9a84c",
  org_editor: "#5B9BD5",
  viewer: "#8fa565",
};

const ROOM_ICONS: Record<string, { icon: LucideIcon; bg: string; color: string }> = {
  general: { icon: Megaphone, bg: "linear-gradient(135deg, #c9a84c 0%, #8b6914 100%)", color: "#14140c" },
  org: { icon: Building2, bg: "linear-gradient(135deg, #3a5a3a 0%, #1a3a1a 100%)", color: "#81c784" },
  direct: { icon: MessageSquare, bg: "linear-gradient(135deg, #3a4a5a 0%, #1a2a3a 100%)", color: "#64b5f6" },
};

// ---------------------------------------------------------------------------
// Rich text: delta links, internal links, URLs
// ---------------------------------------------------------------------------

interface TextSegment {
  type: "text" | "delta" | "internal" | "url";
  value: string;
  label?: string;
  href?: string;
  deltaKind?: string;
}

type DeltaKindInfo = {
  label: string;
  icon: string;
  bg: string;
  color: string;
  border: string;
  glow: string;
};

const DELTA_KINDS: Record<string, DeltaKindInfo> = {
  person: {
    label: "IXD",
    icon: "",
    bg: "linear-gradient(135deg, #1a3a5c 0%, #0d2847 100%)",
    color: "#4fc3f7",
    border: "rgba(79, 195, 247, 0.3)",
    glow: "rgba(79, 195, 247, 0.15)",
  },
  doc: {
    label: "📄",
    icon: "Документ",
    bg: "linear-gradient(135deg, #1a3a5c 0%, #162f4a 100%)",
    color: "#64b5f6",
    border: "rgba(100, 181, 246, 0.35)",
    glow: "rgba(100, 181, 246, 0.18)",
  },
  wiki: {
    label: "📝",
    icon: "Колектив",
    bg: "linear-gradient(135deg, #1a3c3c 0%, #0d2f2f 100%)",
    color: "#4dd0e1",
    border: "rgba(77, 208, 225, 0.35)",
    glow: "rgba(77, 208, 225, 0.18)",
  },
  board: {
    label: "📋",
    icon: "Дошка",
    bg: "linear-gradient(135deg, #2a1f4e 0%, #1a1340 100%)",
    color: "#b39ddb",
    border: "rgba(179, 157, 219, 0.35)",
    glow: "rgba(179, 157, 219, 0.18)",
  },
  table: {
    label: "📊",
    icon: "Таблиця",
    bg: "linear-gradient(135deg, #1a3c2a 0%, #0d2f1a 100%)",
    color: "#81c784",
    border: "rgba(129, 199, 132, 0.35)",
    glow: "rgba(129, 199, 132, 0.18)",
  },
  cal: {
    label: "📅",
    icon: "Подія",
    bg: "linear-gradient(135deg, #3c2a1a 0%, #2f1a0d 100%)",
    color: "#ffb74d",
    border: "rgba(255, 183, 77, 0.35)",
    glow: "rgba(255, 183, 77, 0.18)",
  },
  form: {
    label: "📝",
    icon: "Форма",
    bg: "linear-gradient(135deg, #3c1a2a 0%, #2f0d1a 100%)",
    color: "#f48fb1",
    border: "rgba(244, 143, 177, 0.35)",
    glow: "rgba(244, 143, 177, 0.18)",
  },
};

function classifyDeltaDriveUrl(url: string): { kind: string; label: string } | null {
  if (!url.includes("delta.mil.gov.ua")) return null;
  if (url.includes("/delta-drive/apps/collectives/")) {
    const slug = url.split("/").pop()?.replace(/-/g, " ") ?? "Колектив";
    return { kind: "collective", label: slug.length > 30 ? slug.slice(0, 30) + "…" : slug };
  }
  if (url.includes("/delta-drive/")) {
    return { kind: "file", label: "Delta Drive" };
  }
  return { kind: "link", label: "Delta" };
}

function parseRichText(text: string): TextSegment[] {
  const segments: TextSegment[] = [];
  const pattern = /(delta:(?:(doc|wiki|board|table|cal|form):)?([\wЀ-ӿ\-№]+))|(\/(?:directory|dashboard|data|training|documents|discrepancies|orgs|org\/\d+|import|settings|chat)(?:\?\S*)?)|((https?:\/\/\S+))/g;
  let lastIndex = 0;
  let match;

  while ((match = pattern.exec(text)) !== null) {
    if (match.index > lastIndex) {
      segments.push({ type: "text", value: text.slice(lastIndex, match.index) });
    }
    if (match[1]) {
      const kind = match[2] || "person";
      const name = match[3];
      segments.push({ type: "delta", value: name, label: name, deltaKind: kind });
    } else if (match[4]) {
      segments.push({ type: "internal", value: match[4], href: match[4] });
    } else if (match[5]) {
      const href = match[6];
      const deltaInfo = classifyDeltaDriveUrl(href);
      if (deltaInfo) {
        segments.push({ type: "delta", value: deltaInfo.label, label: deltaInfo.label, deltaKind: deltaInfo.kind === "collective" ? "wiki" : "doc", href });
      } else {
        // Check if it's our own app URL → convert to internal link
        try {
          const u = new URL(href);
          const isOwnDomain = u.hostname === window.location.hostname || u.hostname.endsWith(".striy.pp.ua");
          if (isOwnDomain && u.pathname.match(/^\/(data|dashboard|directory|training|documents|discrepancies|orgs|import|settings|chat)/)) {
            const internal = u.pathname + u.search;
            segments.push({ type: "internal", value: internal, href: internal });
          } else {
            segments.push({ type: "url", value: href, href });
          }
        } catch {
          segments.push({ type: "url", value: href, href });
        }
      }
    }
    lastIndex = match.index + match[0].length;
  }

  if (lastIndex < text.length) {
    segments.push({ type: "text", value: text.slice(lastIndex) });
  }

  return segments.length > 0 ? segments : [{ type: "text", value: text }];
}

const INTERNAL_LINK_NAMES: Record<string, { label: string; icon: string }> = {
  "/directory": { label: "Люди", icon: "👥" },
  "/dashboard": { label: "Аналітика", icon: "📈" },
  "/data": { label: "Дані", icon: "📊" },
  "/training": { label: "Підготовка", icon: "📋" },
  "/documents": { label: "Документи", icon: "📄" },
  "/discrepancies": { label: "Розбіжності", icon: "⚠️" },
  "/orgs": { label: "Підрозділи", icon: "🏢" },
  "/import": { label: "Імпорт", icon: "📥" },
  "/settings": { label: "Налаштування", icon: "⚙️" },
  "/chat": { label: "Чат", icon: "💬" },
};

function RichTextContent({ text }: { text: string }) {
  const segments = useMemo(() => parseRichText(text), [text]);

  return (
    <span>
      {segments.map((seg, i) => {
        switch (seg.type) {
          case "delta": {
            const kind = DELTA_KINDS[seg.deltaKind || "person"] ?? DELTA_KINDS.person;
            const isPerson = seg.deltaKind === "person" || !seg.deltaKind;
            const isExternal = seg.href?.startsWith("http");
            const linkTo = isPerson
              ? `/directory?search=${encodeURIComponent(seg.value)}`
              : "#";
            const titleText = isExternal
              ? `Відкрити в Delta Drive`
              : isPerson
                ? `Відкрити профіль ${seg.value} у Delta`
                : `${kind.icon}: ${seg.value} (IXD Nextcloud)`;
            const badgeStyle = {
              background: kind.bg,
              color: kind.color,
              border: `1px solid ${kind.border}`,
              boxShadow: `0 0 8px ${kind.glow}, inset 0 1px 0 rgba(255,255,255,0.05)`,
              textDecoration: "none" as const,
              letterSpacing: "0.02em",
            };
            const badgeClass = "inline-flex items-center gap-1 rounded-md px-1.5 py-0.5 text-xs font-semibold transition-all duration-200 hover:scale-105";
            const inner = (
              <>
                {isExternal ? (
                  <span style={{ fontSize: "11px" }}>🔗</span>
                ) : isPerson ? (
                  <span style={{ fontSize: "10px", opacity: 0.7 }}>IXD</span>
                ) : (
                  <span style={{ fontSize: "11px" }}>{kind.label}</span>
                )}
                <span style={{ fontWeight: 700 }}>{seg.label}</span>
                {isExternal && (
                  <span style={{ fontSize: "9px", opacity: 0.55, marginLeft: "1px" }}>Delta</span>
                )}
                {!isPerson && !isExternal && (
                  <span style={{ fontSize: "9px", opacity: 0.55, marginLeft: "1px" }}>IXD</span>
                )}
              </>
            );
            if (isExternal) {
              return (
                <a
                  key={i}
                  href={seg.href}
                  target="_blank"
                  rel="noopener noreferrer"
                  onClick={(e) => e.stopPropagation()}
                  className={badgeClass}
                  style={badgeStyle}
                  title={titleText}
                >
                  {inner}
                </a>
              );
            }
            return (
              <Link
                key={i}
                to={linkTo}
                onClick={(e) => {
                  e.stopPropagation();
                  if (!isPerson) {
                    e.preventDefault();
                    toast.info(`${kind.icon} ${seg.value} — відкриється в IXD Nextcloud`);
                  }
                }}
                className={badgeClass}
                style={badgeStyle}
                title={titleText}
              >
                {inner}
              </Link>
            );
          }
          case "internal": {
            const basePath = seg.href?.split("?")[0] ?? "";
            const info = INTERNAL_LINK_NAMES[basePath] ?? { label: seg.value, icon: "🔗" };
            // Build detail suffix from query params
            const qs = seg.href?.includes("?") ? new URLSearchParams(seg.href.split("?")[1]) : null;
            const detail = qs ? [
              qs.get("group") && `#${qs.get("group")}`,
              qs.get("kind") || qs.get("kinds"),
              qs.get("orgs"),
              qs.get("q") && `«${qs.get("q")}»`,
              qs.get("from") && `з ${qs.get("from")}`,
              qs.get("to") && `по ${qs.get("to")}`,
            ].filter(Boolean).join(" ") : "";
            return (
              <Link
                key={i}
                to={seg.href!}
                onClick={(e) => e.stopPropagation()}
                className="inline-flex items-center gap-1 rounded-md px-1.5 py-0.5 text-xs font-semibold transition-all duration-200 hover:scale-105"
                style={{
                  background: "linear-gradient(135deg, rgba(201,168,76,0.15) 0%, rgba(201,168,76,0.08) 100%)",
                  color: "var(--primary)",
                  border: "1px solid rgba(201,168,76,0.25)",
                  textDecoration: "none",
                }}
                title={`Перейти: ${info.label}${detail ? ` ${detail}` : ""}`}
              >
                <span>{info.icon}</span>
                <span>{info.label}{detail ? ` ${detail}` : ""}</span>
              </Link>
            );
          }
          case "url":
            return (
              <a
                key={i}
                href={seg.href}
                target="_blank"
                rel="noopener noreferrer"
                className="underline decoration-dotted underline-offset-2 transition-colors"
                style={{ color: "var(--info)" }}
              >
                {seg.value.replace(/^https?:\/\//, "").slice(0, 40)}
                {(seg.value.length > 47) ? "…" : ""}
              </a>
            );
          default:
            return <span key={i}>{seg.value}</span>;
        }
      })}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Voice recorder hook
// ---------------------------------------------------------------------------

function useVoiceRecorder() {
  const [recording, setRecording] = useState(false);
  const [duration, setDuration] = useState(0);
  const mediaRecorder = useRef<MediaRecorder | null>(null);
  const chunks = useRef<Blob[]>([]);
  const timer = useRef<number>(0);
  const startTime = useRef<number>(0);

  const start = useCallback(async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const mimeType = MediaRecorder.isTypeSupported("audio/webm;codecs=opus")
        ? "audio/webm;codecs=opus"
        : "audio/webm";
      const recorder = new MediaRecorder(stream, { mimeType });
      chunks.current = [];
      recorder.ondataavailable = (e) => {
        if (e.data.size > 0) chunks.current.push(e.data);
      };
      recorder.start(200);
      mediaRecorder.current = recorder;
      startTime.current = Date.now();
      setRecording(true);
      setDuration(0);
      timer.current = window.setInterval(() => {
        setDuration(Math.floor((Date.now() - startTime.current) / 1000));
      }, 500);
    } catch {
      toast.error("Немає доступу до мікрофону");
    }
  }, []);

  const stop = useCallback((): Promise<{ blob: Blob; duration: number }> => {
    return new Promise((resolve) => {
      const recorder = mediaRecorder.current;
      if (!recorder || recorder.state === "inactive") {
        resolve({ blob: new Blob(), duration: 0 });
        return;
      }
      const finalDuration = Math.floor((Date.now() - startTime.current) / 1000);
      recorder.onstop = () => {
        const blob = new Blob(chunks.current, { type: recorder.mimeType });
        recorder.stream.getTracks().forEach((t) => t.stop());
        clearInterval(timer.current);
        setRecording(false);
        setDuration(0);
        resolve({ blob, duration: finalDuration });
      };
      recorder.stop();
    });
  }, []);

  const cancel = useCallback(() => {
    const recorder = mediaRecorder.current;
    if (recorder && recorder.state !== "inactive") {
      recorder.onstop = () => {
        recorder.stream.getTracks().forEach((t) => t.stop());
      };
      recorder.stop();
    }
    clearInterval(timer.current);
    setRecording(false);
    setDuration(0);
    chunks.current = [];
  }, []);

  return { recording, duration, start, stop, cancel };
}

// ---------------------------------------------------------------------------
// Voice player
// ---------------------------------------------------------------------------

function VoicePlayer({
  url,
  durationSec,
  color,
}: {
  url: string;
  durationSec: number | null;
  color: string;
}) {
  const audioRef = useRef<HTMLAudioElement>(null);
  const [playing, setPlaying] = useState(false);
  const [progress, setProgress] = useState(0);
  const [currentTime, setCurrentTime] = useState(0);
  const totalDuration = durationSec ?? 0;

  const toggle = useCallback(() => {
    const audio = audioRef.current;
    if (!audio) return;
    if (playing) {
      audio.pause();
    } else {
      audio.play().catch(() => {});
    }
  }, [playing]);

  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) return;
    const onPlay = () => setPlaying(true);
    const onPause = () => setPlaying(false);
    const onEnded = () => { setPlaying(false); setProgress(0); setCurrentTime(0); };
    const onTime = () => {
      if (audio.duration && isFinite(audio.duration)) {
        setProgress(audio.currentTime / audio.duration);
        setCurrentTime(audio.currentTime);
      }
    };
    audio.addEventListener("play", onPlay);
    audio.addEventListener("pause", onPause);
    audio.addEventListener("ended", onEnded);
    audio.addEventListener("timeupdate", onTime);
    return () => {
      audio.removeEventListener("play", onPlay);
      audio.removeEventListener("pause", onPause);
      audio.removeEventListener("ended", onEnded);
      audio.removeEventListener("timeupdate", onTime);
    };
  }, []);

  const fmtTime = (s: number) => {
    const m = Math.floor(s / 60);
    const sec = Math.floor(s % 60);
    return `${m}:${sec.toString().padStart(2, "0")}`;
  };

  // Generate waveform bars (static visual, animated on play)
  const bars = useMemo(() => {
    const count = 28;
    const result = [];
    for (let i = 0; i < count; i++) {
      const h = 8 + Math.sin(i * 0.7) * 6 + Math.cos(i * 1.3) * 4 + Math.random() * 4;
      result.push(Math.max(4, Math.min(20, h)));
    }
    return result;
  }, []);

  return (
    <div className="flex items-center gap-2 py-1">
      <audio ref={audioRef} src={url} preload="metadata" />
      <button
        onClick={toggle}
        className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full transition-all duration-200 hover:scale-110"
        style={{
          background: `linear-gradient(135deg, ${color}, color-mix(in oklch, ${color} 70%, black))`,
          color: "#fff",
          boxShadow: `0 2px 8px color-mix(in oklch, ${color} 30%, transparent)`,
        }}
      >
        {playing ? <Pause className="h-3.5 w-3.5" /> : <Play className="h-3.5 w-3.5 ml-0.5" />}
      </button>
      <div className="flex flex-1 items-end gap-px" style={{ height: "24px" }}>
        {bars.map((h, i) => {
          const barProgress = i / bars.length;
          const isActive = barProgress <= progress;
          return (
            <div
              key={i}
              className="transition-all duration-150"
              style={{
                width: "3px",
                height: `${h}px`,
                borderRadius: "1.5px",
                background: isActive ? color : "var(--muted-foreground)",
                opacity: isActive ? 1 : 0.25,
                transform: playing && isActive ? "scaleY(1.1)" : "scaleY(1)",
                alignSelf: "center",
              }}
            />
          );
        })}
      </div>
      <span className="text-[11px] tabular-nums text-muted-foreground min-w-[32px] text-right">
        {playing ? fmtTime(currentTime) : fmtTime(totalDuration)}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Media preview in message
// ---------------------------------------------------------------------------

function MediaContent({
  url,
  mime,
  kind,
}: {
  url: string;
  mime: string | null;
  kind: string;
}) {
  const [lightbox, setLightbox] = useState(false);
  const isImage = kind === "image" || mime?.startsWith("image/");
  const isVideo = kind === "video" || mime?.startsWith("video/");

  if (isImage) {
    return (
      <>
        <img
          src={url}
          alt=""
          className="max-w-full sm:max-w-[320px] max-h-[240px] rounded-lg cursor-pointer transition-transform duration-200 hover:scale-[1.02]"
          style={{
            border: "1px solid var(--border)",
            boxShadow: "0 2px 12px rgba(0,0,0,0.3)",
          }}
          onClick={() => setLightbox(true)}
        />
        {lightbox && (
          <div
            className="fixed inset-0 z-[100] flex items-center justify-center bg-black/80 backdrop-blur-sm"
            onClick={() => setLightbox(false)}
          >
            <img
              src={url}
              alt=""
              className="max-w-[90vw] max-h-[90vh] rounded-lg"
              style={{ boxShadow: "0 8px 32px rgba(0,0,0,0.5)" }}
            />
            <button
              className="absolute top-4 right-4 text-white/80 hover:text-white"
              onClick={() => setLightbox(false)}
            >
              <X className="h-8 w-8" />
            </button>
          </div>
        )}
      </>
    );
  }

  if (isVideo) {
    return (
      <video
        src={url}
        controls
        className="max-w-[320px] max-h-[240px] rounded-lg"
        style={{
          border: "1px solid var(--border)",
          boxShadow: "0 2px 12px rgba(0,0,0,0.3)",
        }}
      />
    );
  }

  return null;
}

// ---------------------------------------------------------------------------
// Single message bubble
// ---------------------------------------------------------------------------

function MessageBubble({
  msg,
  isOwn,
  onReply,
  onEdit,
  onDelete,
  onRightClick,
}: {
  msg: ChatMessage;
  isOwn: boolean;
  onReply: (msg: ChatMessage) => void;
  onEdit: (msg: ChatMessage, newBody: string) => void;
  onDelete: (msg: ChatMessage) => void;
  onRightClick?: (e: React.MouseEvent, msg: ChatMessage) => void;
}) {
  const roleColor = ROLE_COLORS[msg.sender_role ?? ""] ?? "var(--muted-foreground)";
  const initials = (msg.sender_callsign ?? msg.sender_label ?? "?").slice(0, 2).toUpperCase();
  const avatarUrl = msg.sender_avatar ? `/api/avatars/${msg.sender_avatar}` : null;

  const isSystem = msg.kind === "system";
  const isDeleted = !!msg.deleted_at;

  const [editing, setEditing] = useState(false);
  const [editText, setEditText] = useState(msg.body);
  const editRef = useRef<HTMLTextAreaElement>(null);

  const ageSec = (Date.now() - new Date(msg.created_at).getTime()) / 1000;
  const canModify = isOwn && ageSec <= 30 && !isDeleted;

  useEffect(() => {
    if (editing && editRef.current) {
      editRef.current.focus();
      editRef.current.setSelectionRange(editRef.current.value.length, editRef.current.value.length);
    }
  }, [editing]);

  if (isDeleted) {
    return (
      <div className="flex justify-center py-1">
        <span className="text-[11px] italic" style={{ color: "var(--muted-foreground)", opacity: 0.5 }}>
          Повідомлення видалено
        </span>
      </div>
    );
  }

  if (isSystem) {
    return (
      <div className="flex justify-center py-1.5">
        <div
          className="rounded-full px-3 py-1 text-xs"
          style={{
            background: "rgba(201,168,76,0.08)",
            color: "var(--muted-foreground)",
            border: "1px solid rgba(201,168,76,0.12)",
          }}
        >
          <RichTextContent text={msg.body} />
        </div>
      </div>
    );
  }

  return (
    <div
      data-msg-id={msg.id}
      onContextMenu={onRightClick ? (e) => onRightClick(e, msg) : undefined}
      className={`group flex gap-2.5 py-1 px-2 rounded-lg transition-colors duration-150 hover:bg-[rgba(201,168,76,0.04)] ${
        isOwn ? "flex-row-reverse" : ""
      }`}
    >
      {/* Avatar */}
      <Link
        to={`/directory?search=${encodeURIComponent(msg.sender_callsign ?? msg.sender_label ?? "")}`}
        className="shrink-0 mt-0.5"
        onClick={(e) => e.stopPropagation()}
      >
        <div
          className="h-9 w-9 rounded-full overflow-hidden flex items-center justify-center text-[11px] font-bold transition-transform duration-200 hover:scale-110"
          style={{
            background: avatarUrl
              ? "transparent"
              : `linear-gradient(135deg, ${roleColor}, color-mix(in oklch, ${roleColor} 60%, transparent))`,
            color: "#fff",
            boxShadow: `0 2px 6px color-mix(in oklch, ${roleColor} 20%, transparent)`,
          }}
        >
          {avatarUrl ? (
            <img src={avatarUrl} alt="" className="h-full w-full object-cover" />
          ) : (
            initials
          )}
        </div>
      </Link>

      {/* Content */}
      <div className={`flex flex-col max-w-[70%] ${isOwn ? "items-end" : ""}`}>
        {/* Name + time + actions */}
        <div className={`flex items-center gap-2 mb-0.5 ${isOwn ? "flex-row-reverse" : ""}`}>
          <Link
            to={`/directory?search=${encodeURIComponent(msg.sender_callsign ?? msg.sender_label ?? "")}`}
            className="text-xs font-semibold transition-colors hover:underline"
            style={{ color: roleColor }}
            onClick={(e) => e.stopPropagation()}
          >
            {msg.sender_callsign ?? msg.sender_label}
          </Link>
          <span className="text-[10px] text-muted-foreground">
            {msg.created_at.replace("T", " ")}
            {msg.updated_at && msg.updated_at !== msg.created_at && (
              <span className="italic ml-1" style={{ opacity: 0.6 }}>(ред.)</span>
            )}
          </span>
          <div className="flex items-center gap-0.5 opacity-0 group-hover:opacity-60 hover:!opacity-100 transition-opacity">
            <button onClick={() => onReply(msg)} title="Відповісти">
              <Reply className="h-3 w-3" />
            </button>
            {canModify && (
              <>
                <button
                  data-edit-trigger
                  onClick={() => { setEditing(true); setEditText(msg.body); }}
                  title="Редагувати"
                >
                  <Pencil className="h-3 w-3" />
                </button>
                <button
                  onClick={() => onDelete(msg)}
                  title="Видалити"
                  className="hover:text-red-400"
                >
                  <Trash2 className="h-3 w-3" />
                </button>
              </>
            )}
          </div>
        </div>

        {/* Reply quote */}
        {msg.reply_to_id && msg.reply_preview && (
          <div
            className="text-[11px] px-2 py-1 mb-1 rounded-md border-l-2"
            style={{
              background: "rgba(201,168,76,0.06)",
              borderLeftColor: "var(--primary)",
              color: "var(--muted-foreground)",
              maxWidth: "100%",
            }}
          >
            {msg.reply_preview.length > 60
              ? msg.reply_preview.slice(0, 60) + "…"
              : msg.reply_preview}
          </div>
        )}

        {/* Bubble */}
        <div
          className="rounded-2xl px-3 py-2 text-sm leading-relaxed"
          style={{
            background: isOwn
              ? "linear-gradient(135deg, rgba(201,168,76,0.18) 0%, rgba(201,168,76,0.10) 100%)"
              : "var(--card)",
            border: isOwn
              ? "1px solid rgba(201,168,76,0.25)"
              : "1px solid var(--border)",
            borderTopLeftRadius: isOwn ? "16px" : "4px",
            borderTopRightRadius: isOwn ? "4px" : "16px",
            boxShadow: "0 1px 3px rgba(0,0,0,0.1)",
          }}
        >
          {/* Voice message */}
          {msg.kind === "voice" && msg.media_url && (
            <VoicePlayer
              url={msg.media_url}
              durationSec={msg.media_duration_sec}
              color={isOwn ? "var(--primary)" : roleColor}
            />
          )}

          {/* Image / Video */}
          {(msg.kind === "image" || msg.kind === "video") && msg.media_url && (
            <MediaContent url={msg.media_url} mime={msg.media_mime} kind={msg.kind} />
          )}

          {/* Text body — editing or display */}
          {editing ? (
            <div className="flex flex-col gap-1">
              <textarea
                ref={editRef}
                value={editText}
                onChange={(e) => setEditText(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && !e.shiftKey) {
                    e.preventDefault();
                    if (editText.trim() && editText !== msg.body) {
                      onEdit(msg, editText.trim());
                    }
                    setEditing(false);
                  }
                  if (e.key === "Escape") setEditing(false);
                }}
                className="w-full rounded-md border px-2 py-1 text-sm resize-none bg-transparent"
                style={{ borderColor: "var(--border)", color: "var(--foreground)", minHeight: "2rem" }}
                rows={1}
              />
              <div className="flex gap-1 justify-end">
                <button
                  className="text-[10px] px-1.5 py-0.5 rounded hover:bg-accent/50"
                  onClick={() => setEditing(false)}
                >
                  Скасувати
                </button>
                <button
                  className="text-[10px] px-1.5 py-0.5 rounded font-semibold"
                  style={{ color: "var(--primary)" }}
                  onClick={() => {
                    if (editText.trim() && editText !== msg.body) {
                      onEdit(msg, editText.trim());
                    }
                    setEditing(false);
                  }}
                >
                  <Check className="h-3 w-3 inline mr-0.5" />
                  Зберегти
                </button>
              </div>
            </div>
          ) : msg.body && msg.kind !== "voice" ? (
            <div>
              <RichTextContent text={msg.body} />
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Room sidebar helpers
// ---------------------------------------------------------------------------

const ROOM_GROUPS: { kind: string; label: string; icon: LucideIcon }[] = [
  { kind: "general", label: "Загальні", icon: Megaphone },
  { kind: "org", label: "Підрозділи", icon: Building2 },
  { kind: "direct", label: "Особисті", icon: MessageSquare },
];

function formatRelativeTime(iso: string | null): string {
  if (!iso) return "";
  const diff = Date.now() - new Date(iso).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "щойно";
  if (mins < 60) return `${mins}хв`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}год`;
  const days = Math.floor(hrs / 24);
  if (days === 1) return "вчора";
  if (days < 7) return `${days}д`;
  return new Date(iso).toLocaleDateString("uk-UA", { day: "numeric", month: "short" });
}

function RoomItem({
  room,
  active,
  onClick,
  onRightClick,
}: {
  room: ChatRoom;
  active: boolean;
  onClick: () => void;
  onRightClick?: (e: React.MouseEvent, room: ChatRoom) => void;
}) {
  const roomIcon = ROOM_ICONS[room.kind] ?? ROOM_ICONS.direct;
  const IconComp = roomIcon.icon;
  const unread = room.unread_count ?? 0;
  const timeStr = formatRelativeTime(room.last_message_at);

  return (
    <button
      onClick={onClick}
      onContextMenu={onRightClick ? (e) => onRightClick(e, room) : undefined}
      className="group flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-left transition-all duration-200 hover:bg-accent/50"
      style={{
        background: active
          ? "linear-gradient(135deg, rgba(201,168,76,0.15), rgba(201,168,76,0.08))"
          : undefined,
        borderLeft: active ? "3px solid var(--primary)" : "3px solid transparent",
      }}
    >
      <div
        className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg"
        style={{
          background: active ? roomIcon.bg : "var(--muted)",
          boxShadow: active ? `0 2px 8px ${roomIcon.color}33` : undefined,
          transition: "all 0.2s",
        }}
      >
        <IconComp
          className="h-4.5 w-4.5"
          style={{ color: active ? roomIcon.color : "var(--muted-foreground)" }}
        />
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-1">
          <span
            className="font-semibold text-[13px] truncate flex-1"
            style={{ color: active ? "var(--primary)" : "var(--foreground)" }}
          >
            {room.name}
          </span>
          {timeStr && (
            <span
              className="text-[10px] shrink-0 tabular-nums"
              style={{ color: unread > 0 ? "var(--primary)" : "var(--muted-foreground)" }}
            >
              {timeStr}
            </span>
          )}
        </div>
        <div className="flex items-center gap-1 mt-0.5">
          {room.last_message_body ? (
            <p
              className="text-[11px] truncate flex-1"
              style={{
                color: "var(--muted-foreground)",
                fontWeight: unread > 0 ? 500 : 400,
              }}
            >
              {room.last_message_body.slice(0, 50)}
            </p>
          ) : (
            <p className="text-[11px] flex-1 italic" style={{ color: "var(--muted-foreground)", opacity: 0.5 }}>
              Немає повідомлень
            </p>
          )}
          {unread > 0 && (
            <span
              className="flex h-[18px] min-w-[18px] items-center justify-center rounded-full px-1 text-[10px] font-bold shrink-0"
              style={{
                background: "linear-gradient(135deg, #c9a84c, #b8912c)",
                color: "#14140c",
                boxShadow: "0 1px 4px rgba(201,168,76,0.3)",
              }}
            >
              {unread > 99 ? "99+" : unread}
            </span>
          )}
        </div>
      </div>
    </button>
  );
}

function RoomGroup({
  group,
  rooms,
  activeRoom,
  onSelect,
  onRoomRightClick,
}: {
  group: { kind: string; label: string; icon: LucideIcon };
  rooms: ChatRoom[];
  activeRoom: number | null;
  onSelect: (id: number) => void;
  onRoomRightClick?: (e: React.MouseEvent, room: ChatRoom) => void;
}) {
  const [open, setOpen] = useState(true);
  const totalUnread = rooms.reduce((s, r) => s + (r.unread_count ?? 0), 0);

  if (rooms.length === 0) return null;

  return (
    <div>
      <button
        onClick={() => setOpen(!open)}
        className="flex w-full items-center gap-2 px-3 py-2 text-left group/header"
        style={{ marginTop: "4px" }}
      >
        <ChevronRight
          className="h-3 w-3 shrink-0 transition-transform duration-200"
          style={{
            transform: open ? "rotate(90deg)" : "rotate(0deg)",
            color: "var(--primary)",
            opacity: 0.6,
          }}
        />
        <group.icon
          className="h-3 w-3 shrink-0"
          style={{ color: "color-mix(in srgb, var(--primary) 70%, var(--muted-foreground))" }}
        />
        <span
          className="text-[10px] font-bold uppercase tracking-[0.1em]"
          style={{ color: "color-mix(in srgb, var(--primary) 70%, var(--muted-foreground))" }}
        >
          {group.label}
        </span>
        <span
          className="ml-auto flex h-[18px] min-w-[18px] items-center justify-center rounded-md px-1 text-[10px] font-medium"
          style={{
            background: "color-mix(in srgb, var(--muted) 80%, transparent)",
            color: "var(--muted-foreground)",
          }}
        >
          {rooms.length}
        </span>
        {totalUnread > 0 && (
          <span
            className="flex h-[18px] min-w-[18px] items-center justify-center rounded-full px-1 text-[9px] font-bold"
            style={{ background: "var(--primary)", color: "#14140c" }}
          >
            {totalUnread}
          </span>
        )}
      </button>
      {open && (
        <div className="space-y-0.5 mt-0.5">
          {rooms.map((room) => (
            <RoomItem
              key={room.id}
              room={room}
              active={room.id === activeRoom}
              onClick={() => onSelect(room.id)}
              onRightClick={onRoomRightClick}
            />
          ))}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Recording indicator bar
// ---------------------------------------------------------------------------

function RecordingBar({
  duration,
  onStop,
  onCancel,
}: {
  duration: number;
  onStop: () => void;
  onCancel: () => void;
}) {
  const fmtTime = (s: number) => {
    const m = Math.floor(s / 60);
    const sec = s % 60;
    return `${m}:${sec.toString().padStart(2, "0")}`;
  };

  return (
    <div
      className="flex items-center gap-3 px-4 py-3 rounded-2xl animate-in fade-in duration-300"
      style={{
        background: "linear-gradient(135deg, rgba(217, 83, 79, 0.15), rgba(217, 83, 79, 0.08))",
        border: "1px solid rgba(217, 83, 79, 0.3)",
      }}
    >
      <div className="flex items-center gap-2 flex-1">
        <div className="relative">
          <Mic className="h-5 w-5" style={{ color: "#D9534F" }} />
          <span
            className="absolute -top-0.5 -right-0.5 h-2.5 w-2.5 rounded-full animate-pulse"
            style={{ background: "#D9534F", boxShadow: "0 0 6px #D9534F" }}
          />
        </div>
        <span className="text-sm font-semibold tabular-nums" style={{ color: "#D9534F" }}>
          {fmtTime(duration)}
        </span>
        {/* Animated recording waveform */}
        <div className="flex items-center gap-0.5">
          {Array.from({ length: 16 }).map((_, i) => (
            <div
              key={i}
              className="rounded-full"
              style={{
                width: "2px",
                height: `${6 + Math.sin(Date.now() / 200 + i * 0.5) * 6}px`,
                background: "#D9534F",
                opacity: 0.5 + Math.sin(Date.now() / 300 + i) * 0.3,
                transition: "height 0.15s ease",
              }}
            />
          ))}
        </div>
      </div>
      <Button
        variant="ghost"
        size="sm"
        className="h-8 px-3 text-xs"
        onClick={onCancel}
      >
        <X className="mr-1 h-3 w-3" />
        Скасувати
      </Button>
      <Button
        size="sm"
        className="h-8 px-4 text-xs font-semibold"
        onClick={onStop}
        style={{
          background: "linear-gradient(135deg, #D9534F, #c9443f)",
          color: "#fff",
        }}
      >
        <Square className="mr-1 h-3 w-3" />
        Надіслати
      </Button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main Chat Page
// ---------------------------------------------------------------------------

export function ChatPage() {
  const { user } = useAuth();
  const [rooms, setRooms] = useState<ChatRoom[]>([]);
  const [chatParams, setChatParams] = useSearchParams();
  const initialRoom = chatParams.get("room") ? Number(chatParams.get("room")) : null;
  const [activeRoom, setActiveRoom] = useState<number | null>(initialRoom);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadingMsgs, setLoadingMsgs] = useState(false);
  const [text, setText] = useState(chatParams.get("prefill") ?? "");
  const [replyTo, setReplyTo] = useState<ChatMessage | null>(null);
  const [mobileSidebar, setMobileSidebar] = useState(true);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const eventSourceRef = useRef<EventSource | null>(null);
  const ctxMenu = useContextMenu();
  const voice = useVoiceRecorder();

  // Clear prefill/room from URL and focus input
  useEffect(() => {
    if (chatParams.has("prefill") || chatParams.has("room")) {
      setChatParams({}, { replace: true });
      if (chatParams.has("prefill")) {
        setTimeout(() => inputRef.current?.focus(), 300);
      }
    }
  }, []);

  // Load rooms
  useEffect(() => {
    api.get<ChatRoom[]>("/chat/rooms")
      .then((data) => {
        setRooms(data);
        if (data.length > 0 && activeRoom === null) {
          setActiveRoom(data[0].id);
        }
        if (activeRoom !== null || data.length > 0) {
          setMobileSidebar(false);
        }
      })
      .catch(() => toast.error("Помилка завантаження чатів"))
      .finally(() => setLoading(false));
  }, []);

  // Load messages for active room
  useEffect(() => {
    if (activeRoom === null) return;
    setLoadingMsgs(true);
    api.get<ChatMessage[]>(`/chat/rooms/${activeRoom}/messages?limit=80`)
      .then((data) => {
        setMessages(data.reverse());
        setTimeout(() => scrollToBottom(), 100);
      })
      .catch(() => {})
      .finally(() => setLoadingMsgs(false));
  }, [activeRoom]);

  // SSE for real-time messages
  useEffect(() => {
    if (activeRoom === null) return;
    const es = new EventSource(`/api/chat/rooms/${activeRoom}/stream`);
    eventSourceRef.current = es;

    es.addEventListener("message", (e) => {
      try {
        const msg: ChatMessage = JSON.parse(e.data);
        setMessages((prev) => {
          if (prev.find((m) => m.id === msg.id)) return prev;
          return [...prev, msg];
        });
        setTimeout(() => scrollToBottom(), 50);

        // Update room list unread
        setRooms((prev) =>
          prev.map((r) =>
            r.id === msg.room_id
              ? { ...r, last_message_body: msg.body, last_message_at: msg.created_at }
              : r
          )
        );
      } catch {}
    });

    return () => {
      es.close();
      eventSourceRef.current = null;
    };
  }, [activeRoom]);

  const scrollToBottom = useCallback(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, []);

  const handleEditMessage = useCallback(async (msg: ChatMessage, newBody: string) => {
    try {
      await api.put(`/chat/messages/${msg.id}`, { body: newBody });
      setMessages((prev) => prev.map((m) =>
        m.id === msg.id ? { ...m, body: newBody, updated_at: new Date().toISOString() } : m
      ));
    } catch (e: any) {
      const status = e?.response?.status ?? e?.status;
      if (status === 410) {
        toast.error("Час для редагування вичерпано (30 сек)");
      } else {
        toast.error("Не вдалося відредагувати");
      }
    }
  }, []);

  const handleDeleteMessage = useCallback(async (msg: ChatMessage) => {
    try {
      await api.delete(`/chat/messages/${msg.id}`);
      setMessages((prev) => prev.map((m) =>
        m.id === msg.id ? { ...m, deleted_at: new Date().toISOString() } : m
      ));
    } catch (e: any) {
      const status = e?.response?.status ?? e?.status;
      if (status === 410) {
        toast.error("Час для видалення вичерпано (30 сек)");
      } else {
        toast.error("Не вдалося видалити");
      }
    }
  }, []);

  const handleMsgContextMenu = useCallback((e: React.MouseEvent, msg: ChatMessage) => {
    const isOwn = msg.sender_id === user?.user_id;
    const ageSec = (Date.now() - new Date(msg.created_at).getTime()) / 1000;
    const canModify = isOwn && ageSec <= 30 && !msg.deleted_at;

    const items: ContextMenuEntry[] = [
      {
        label: "Відповісти",
        icon: <Reply className="h-3.5 w-3.5" />,
        onClick: () => setReplyTo(msg),
      },
      {
        label: "Копіювати текст",
        icon: <Copy className="h-3.5 w-3.5" />,
        disabled: !msg.body,
        onClick: () => {
          navigator.clipboard.writeText(msg.body);
          toast.success("Скопійовано");
        },
      },
    ];

    if (canModify) {
      items.push({ separator: true });
      items.push({
        label: "Редагувати",
        icon: <Pencil className="h-3.5 w-3.5" />,
        shortcut: "30с",
        onClick: () => {
          // Trigger edit mode on the message bubble via a custom event
          const el = document.querySelector(`[data-msg-id="${msg.id}"] [data-edit-trigger]`) as HTMLButtonElement;
          el?.click();
        },
      });
      items.push({
        label: "Видалити",
        icon: <Trash2 className="h-3.5 w-3.5" />,
        variant: "destructive",
        onClick: () => handleDeleteMessage(msg),
      });
    }

    ctxMenu.open(e, items);
  }, [user, ctxMenu, handleDeleteMessage]);

  // Send text message
  const sendMessage = useCallback(async () => {
    if (!text.trim() || activeRoom === null) return;
    const body = text.trim();
    setText("");
    const replyId = replyTo?.id ?? null;
    setReplyTo(null);

    try {
      await api.post(`/chat/rooms/${activeRoom}/messages`, {
        body,
        kind: "text",
        reply_to_id: replyId,
      });
    } catch {
      toast.error("Не вдалося надіслати");
      setText(body);
    }
  }, [text, activeRoom, replyTo]);

  // Upload media file
  const uploadAndSend = useCallback(
    async (file: File) => {
      if (activeRoom === null) return;
      const formData = new FormData();
      formData.append("file", file);

      try {
        const res = await fetch("/api/chat/media", {
          method: "POST",
          body: formData,
          credentials: "same-origin",
        });
        if (!res.ok) throw new Error();
        const data = await res.json();

        const kind = file.type.startsWith("image/")
          ? "image"
          : file.type.startsWith("video/")
            ? "video"
            : "file";

        await api.post(`/chat/rooms/${activeRoom}/messages`, {
          kind,
          media_url: data.url,
          media_mime: data.mime,
          body: file.name,
        });
      } catch {
        toast.error("Помилка завантаження файлу");
      }
    },
    [activeRoom]
  );

  // Send voice message
  const sendVoice = useCallback(async () => {
    if (activeRoom === null) return;
    const { blob, duration } = await voice.stop();
    if (blob.size === 0) return;

    const formData = new FormData();
    formData.append("file", blob, `voice_${Date.now()}.webm`);

    try {
      const res = await fetch("/api/chat/media", {
        method: "POST",
        body: formData,
        credentials: "same-origin",
      });
      if (!res.ok) throw new Error();
      const data = await res.json();

      await api.post(`/chat/rooms/${activeRoom}/messages`, {
        kind: "voice",
        media_url: data.url,
        media_mime: data.mime,
        media_duration_sec: duration,
        body: "",
      });
    } catch {
      toast.error("Помилка надсилання голосового");
    }
  }, [activeRoom, voice]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        sendMessage();
      }
    },
    [sendMessage]
  );

  const handleFileSelect = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const file = e.target.files?.[0];
      if (file) uploadAndSend(file);
      e.target.value = "";
    },
    [uploadAndSend]
  );

  const currentRoom = rooms.find((r) => r.id === activeRoom);

  // Group messages by date
  const groupedMessages = useMemo(() => {
    const groups: { date: string; messages: ChatMessage[] }[] = [];
    let currentDate = "";
    for (const msg of messages) {
      const date = msg.created_at.split("T")[0];
      if (date !== currentDate) {
        currentDate = date;
        groups.push({ date, messages: [msg] });
      } else {
        groups[groups.length - 1].messages.push(msg);
      }
    }
    return groups;
  }, [messages]);

  const formatDate = (dateStr: string) => {
    const [y, m, d] = dateStr.split("-").map(Number);
    const today = new Date();
    const date = new Date(y, m - 1, d);
    const diff = Math.floor((today.getTime() - date.getTime()) / 86400000);
    if (diff === 0) return "Сьогодні";
    if (diff === 1) return "Вчора";
    const months = ["січня", "лютого", "березня", "квітня", "травня", "червня",
      "липня", "серпня", "вересня", "жовтня", "листопада", "грудня"];
    return `${d} ${months[m - 1]} ${y}`;
  };

  if (loading) {
    return (
      <div className="flex h-[calc(100vh-4rem)] items-center justify-center">
        <Skeleton className="h-8 w-48" />
      </div>
    );
  }

  return (
    <div
      className="flex h-[calc(100vh-4.5rem)] overflow-hidden rounded-xl"
      style={{
        border: "1px solid var(--border)",
        background: "var(--background)",
      }}
    >
      {/* ── Sidebar ── */}
      <div
        className={`flex flex-col border-r border-border shrink-0 transition-all duration-300 ${
          mobileSidebar ? "w-full md:w-72" : "hidden md:flex md:w-72"
        }`}
        style={{ background: "var(--card)" }}
      >
        {/* Sidebar header */}
        <div
          className="flex items-center gap-2 px-4 py-3 border-b"
          style={{ borderColor: "var(--border)" }}
        >
          <MessageCircle className="h-5 w-5" style={{ color: "var(--primary)" }} />
          <h2
            className="font-bold text-sm uppercase tracking-wider"
            style={{ fontFamily: "var(--font-heading)", color: "var(--foreground)" }}
          >
            Чати
          </h2>
          <Badge variant="secondary" className="ml-auto text-[10px]">
            {rooms.length}
          </Badge>
        </div>

        {/* Room list — grouped by kind */}
        <div className="flex-1 overflow-y-auto p-2 space-y-3">
          {ROOM_GROUPS.map((group) => {
            const groupRooms = rooms.filter((r) => r.kind === group.kind);
            return (
              <RoomGroup
                key={group.kind}
                group={group}
                rooms={groupRooms}
                activeRoom={activeRoom}
                onSelect={(id) => {
                  setActiveRoom(id);
                  setMobileSidebar(false);
                }}
                onRoomRightClick={(e, room) => {
                  ctxMenu.open(e, [
                    {
                      label: "Відкрити",
                      icon: <MessageCircle className="h-3.5 w-3.5" />,
                      onClick: () => { setActiveRoom(room.id); setMobileSidebar(false); },
                    },
                    {
                      label: "Копіювати назву",
                      icon: <Copy className="h-3.5 w-3.5" />,
                      onClick: () => { navigator.clipboard.writeText(room.name); toast.success("Скопійовано"); },
                    },
                  ]);
                }}
              />
            );
          })}
        </div>
      </div>

      {/* ── Main chat area ── */}
      <div className="flex flex-1 flex-col min-w-0">
        {activeRoom === null ? (
          <div className="flex-1 flex items-center justify-center text-muted-foreground">
            <div className="text-center">
              <MessageCircle className="h-16 w-16 mx-auto mb-4 opacity-20" />
              <p className="text-lg font-semibold">Оберіть чат</p>
              <p className="text-sm">Виберіть кімнату зліва для початку спілкування</p>
            </div>
          </div>
        ) : (
          <>
            {/* Room header */}
            <div
              className="flex items-center gap-3 px-4 py-3 border-b shrink-0"
              style={{
                borderColor: "var(--border)",
                background: "linear-gradient(180deg, var(--card) 0%, var(--background) 100%)",
              }}
            >
              <button
                className="md:hidden shrink-0"
                onClick={() => setMobileSidebar(true)}
              >
                <ChevronLeft className="h-5 w-5" />
              </button>
              {(() => {
                const ri = ROOM_ICONS[currentRoom?.kind ?? "direct"] ?? ROOM_ICONS.direct;
                const Ic = ri.icon;
                return (
                  <div
                    className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg"
                    style={{ background: ri.bg, boxShadow: `0 2px 8px ${ri.color}33` }}
                  >
                    <Ic className="h-5 w-5" style={{ color: ri.color }} />
                  </div>
                );
              })()}
              <div className="flex-1 min-w-0">
                <h3
                  className="font-bold text-sm truncate"
                  style={{ fontFamily: "var(--font-heading)", textTransform: "uppercase", letterSpacing: "0.04em" }}
                >
                  {currentRoom?.name}
                </h3>
                <div className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
                  <Users className="h-3 w-3" />
                  <span>{currentRoom?.member_count ?? 0} учасників</span>
                </div>
              </div>
            </div>

            {/* Messages area */}
            <div
              className="flex-1 overflow-y-auto px-2 py-3 space-y-1"
              style={{ scrollBehavior: "smooth" }}
            >
              {loadingMsgs ? (
                <div className="flex flex-col gap-3 p-4">
                  {Array.from({ length: 5 }).map((_, i) => (
                    <div key={i} className={`flex gap-2 ${i % 2 ? "flex-row-reverse" : ""}`}>
                      <Skeleton className="h-9 w-9 rounded-full shrink-0" />
                      <Skeleton className={`h-12 rounded-2xl ${i % 2 ? "w-48" : "w-64"}`} />
                    </div>
                  ))}
                </div>
              ) : (
                groupedMessages.map((group) => (
                  <div key={group.date}>
                    {/* Date separator */}
                    <div className="flex items-center gap-3 py-2">
                      <div className="flex-1 h-px" style={{ background: "var(--border)" }} />
                      <span
                        className="text-[10px] font-semibold uppercase tracking-wider px-2 py-0.5 rounded-full"
                        style={{
                          background: "rgba(201,168,76,0.08)",
                          color: "var(--muted-foreground)",
                          border: "1px solid rgba(201,168,76,0.12)",
                        }}
                      >
                        {formatDate(group.date)}
                      </span>
                      <div className="flex-1 h-px" style={{ background: "var(--border)" }} />
                    </div>
                    {group.messages.map((msg) => (
                      <MessageBubble
                        key={msg.id}
                        msg={msg}
                        isOwn={msg.sender_id === user?.user_id}
                        onReply={setReplyTo}
                        onEdit={handleEditMessage}
                        onDelete={handleDeleteMessage}
                        onRightClick={handleMsgContextMenu}
                      />
                    ))}
                  </div>
                ))
              )}
              <div ref={messagesEndRef} />
            </div>

            {/* Reply preview */}
            {replyTo && (
              <div
                className="flex items-center gap-2 px-4 py-2 border-t"
                style={{
                  borderColor: "var(--border)",
                  background: "rgba(201,168,76,0.04)",
                }}
              >
                <Reply className="h-4 w-4 text-muted-foreground shrink-0" />
                <div className="flex-1 min-w-0">
                  <span className="text-xs font-semibold" style={{ color: "var(--primary)" }}>
                    {replyTo.sender_callsign ?? replyTo.sender_label}
                  </span>
                  <p className="text-xs text-muted-foreground truncate">
                    {replyTo.body.slice(0, 60)}
                  </p>
                </div>
                <button onClick={() => setReplyTo(null)}>
                  <X className="h-4 w-4 text-muted-foreground" />
                </button>
              </div>
            )}

            {/* Composer */}
            <div
              className="shrink-0 px-4 py-3 border-t"
              style={{
                borderColor: "var(--border)",
                background: "linear-gradient(180deg, var(--card) 0%, color-mix(in oklch, var(--card) 95%, var(--background)) 100%)",
              }}
            >
              {voice.recording ? (
                <RecordingBar
                  duration={voice.duration}
                  onStop={sendVoice}
                  onCancel={voice.cancel}
                />
              ) : (
                <div
                  className="flex items-end gap-2 rounded-2xl px-3 py-2 transition-all duration-300 focus-within:shadow-[0_0_0_1px_rgba(201,168,76,0.4),0_0_12px_rgba(201,168,76,0.12)]"
                  style={{
                    background: "var(--background)",
                    border: "1px solid var(--border)",
                  }}
                >
                  {/* Left action group */}
                  <div className="flex items-center gap-0.5 shrink-0 pb-0.5">
                    <input
                      ref={fileInputRef}
                      type="file"
                      accept="image/*,video/*"
                      className="hidden"
                      onChange={handleFileSelect}
                    />
                    <button
                      onClick={() => fileInputRef.current?.click()}
                      title="Додати фото/відео"
                      className="flex h-8 w-8 items-center justify-center rounded-lg transition-all duration-200 hover:bg-[rgba(201,168,76,0.1)]"
                      style={{ color: "var(--muted-foreground)" }}
                      onMouseEnter={(e) => { e.currentTarget.style.color = "var(--primary)"; }}
                      onMouseLeave={(e) => { e.currentTarget.style.color = "var(--muted-foreground)"; }}
                    >
                      <Paperclip className="h-[18px] w-[18px]" />
                    </button>
                  </div>

                  {/* Textarea */}
                  <textarea
                    ref={inputRef}
                    placeholder="Написати повідомлення…"
                    value={text}
                    onChange={(e) => {
                      setText(e.target.value);
                      const el = e.target;
                      el.style.height = "auto";
                      el.style.height = Math.min(el.scrollHeight, 140) + "px";
                    }}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" && !e.shiftKey) {
                        e.preventDefault();
                        sendMessage();
                        const el = e.target as HTMLTextAreaElement;
                        el.style.height = "auto";
                      }
                    }}
                    rows={1}
                    className="flex-1 resize-none bg-transparent py-1.5 text-sm leading-relaxed outline-none placeholder:text-muted-foreground/60"
                    style={{
                      minHeight: "32px",
                      maxHeight: "140px",
                      scrollbarWidth: "thin",
                      scrollbarColor: "rgba(201,168,76,0.2) transparent",
                    }}
                  />

                  {/* Right action group */}
                  <div className="flex items-center gap-0.5 shrink-0 pb-0.5">
                    {text.trim() ? (
                      <button
                        onClick={() => {
                          sendMessage();
                          if (inputRef.current) inputRef.current.style.height = "auto";
                        }}
                        className="flex h-9 w-9 items-center justify-center rounded-xl transition-all duration-200 hover:scale-110 active:scale-95"
                        style={{
                          background: "linear-gradient(135deg, var(--primary), color-mix(in oklch, var(--primary) 75%, #000))",
                          color: "var(--primary-foreground)",
                          boxShadow: "0 2px 10px rgba(201,168,76,0.35), inset 0 1px 0 rgba(255,255,255,0.1)",
                        }}
                      >
                        <Send className="h-4 w-4" />
                      </button>
                    ) : (
                      <button
                        onClick={voice.start}
                        title="Голосове повідомлення"
                        className="flex h-8 w-8 items-center justify-center rounded-lg transition-all duration-200 hover:bg-[rgba(217,83,79,0.12)]"
                        style={{ color: "var(--muted-foreground)" }}
                        onMouseEnter={(e) => { e.currentTarget.style.color = "#D9534F"; }}
                        onMouseLeave={(e) => { e.currentTarget.style.color = "var(--muted-foreground)"; }}
                      >
                        <Mic className="h-[18px] w-[18px]" />
                      </button>
                    )}
                  </div>
                </div>
              )}
            </div>
          </>
        )}
      </div>
      <ContextMenuPortal state={ctxMenu.state} onClose={ctxMenu.close} />
    </div>
  );
}
