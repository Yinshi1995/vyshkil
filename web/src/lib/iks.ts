import { useEffect, useState } from "react";

/**
 * Рейка ІКС (docs/spec/components/iks-rail.md): навігація між застосунками ІКС.
 * Дані й поведінка — спільний контракт ІКС; вигляд — наш.
 */

const LAB_SUFFIX = ".lab.okpivden.uk";

/** Домен лабораторії (`lab.okpivden.uk`) або `null` — тоді рейки немає. */
function detectLab(): string | null {
  const host = location.hostname;
  if (host.endsWith(LAB_SUFFIX)) return host.slice(host.indexOf(".") + 1);
  // Локальна розробка: рейку видно, щоб її можна було перевірити (модулі можуть не завантажитись).
  if (import.meta.env.DEV) return "lab.okpivden.uk";
  return null;
}

export const LAB = detectLab();

export const iksUrls = LAB
  ? {
      home: `https://apps.${LAB}/`,
      emblem: `https://apps.${LAB}/emblem.png`,
      profile: `https://id.${LAB}/settings/account`,
      logoutAll: `https://gate.${LAB}/logout`,
      /** Іконка модуля під тему: кольори вшиті в SVG (набір dark/ і light/), без фільтрів. */
      icon: (name: string, theme: "dark" | "light") => `https://apps.${LAB}/icons/web/${theme}/${encodeURIComponent(name)}.svg`,
    }
  : null;

export interface IksModule {
  name: string;
  url: string;
  title: string;
  description?: string;
  current: boolean;
}

interface IndexRoute {
  name?: unknown;
  url?: unknown;
  title?: unknown;
  description?: unknown;
}

function isCurrent(url: string, name: string): boolean {
  try {
    if (new URL(url).host === location.host) return true;
  } catch {
    return false;
  }
  // Локально адреса не *.lab — позначаємо себе за іменем, щоб було видно поточний.
  return import.meta.env.DEV && name === "vyshkil";
}

let cached: Promise<IksModule[]> | null = null;

function load(): Promise<IksModule[]> {
  if (!LAB) return Promise.resolve([]);
  if (!cached) {
    cached = fetch(`https://index.${LAB}/`, { credentials: "omit" })
      .then((r) => (r.ok ? r.json() : { routes: [] }))
      .then((data: { routes?: IndexRoute[] }) =>
        (Array.isArray(data.routes) ? data.routes : []).flatMap((r) => {
          const name = typeof r.name === "string" ? r.name : "";
          const url = typeof r.url === "string" ? r.url : "";
          if (!name || name === "apps" || name === "id" || !url.startsWith("https://")) return [];
          const title = typeof r.title === "string" && r.title.trim() ? r.title.trim() : name;
          const description =
            typeof r.description === "string" && r.description.trim() ? r.description.trim() : undefined;
          return [{ name, url, title, description, current: isCurrent(url, name) }];
        }),
      )
      // Не завантажилось — рейка лише з гербом і аватаром, без помилок для людини.
      .catch(() => []);
  }
  return cached;
}

/** Модулі ІКС, відкриті цьому пристрою; `[]` поки вантажиться або якщо не вдалось. */
export function useIksModules(): IksModule[] {
  const [modules, setModules] = useState<IksModule[]>([]);
  useEffect(() => {
    let alive = true;
    load().then((m) => {
      if (alive) setModules(m);
    });
    return () => {
      alive = false;
    };
  }, []);
  return modules;
}
