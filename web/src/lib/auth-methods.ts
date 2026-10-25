import { useEffect, useState } from "react";
import { api } from "@/api/client";

/** Способи входу, увімкнені на сервері (`GET /api/auth/methods`, 12-auth.md §2.5). */
export interface AuthMethods {
  /** «Увійти через ІКС» (OIDC, Pocket ID) налаштовано. */
  sso: boolean;
  /** Аварійний вхід за паролем (`PASSWORD_LOGIN=1`): форма пароля, зміна/скидання пароля, запит акаунта. */
  password: boolean;
  /** Адреса ІКС — для посилання на профіль. */
  sso_url: string | null;
}

// Один запит на сесію вкладки: конфіг сервера між навігаціями не змінюється.
let cached: Promise<AuthMethods> | null = null;

function load(): Promise<AuthMethods> {
  if (!cached) {
    cached = api.get<AuthMethods>("/auth/methods").catch(() => {
      cached = null;
      // Сервер недоступний / старий бекенд — показуємо лише ІКС, без парольних дій.
      return { sso: true, password: false, sso_url: null };
    });
  }
  return cached;
}

/** `null` — ще завантажується. */
export function useAuthMethods(): AuthMethods | null {
  const [methods, setMethods] = useState<AuthMethods | null>(null);
  useEffect(() => {
    let alive = true;
    load().then((m) => { if (alive) setMethods(m); });
    return () => { alive = false; };
  }, []);
  return methods;
}
