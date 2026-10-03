import { mockFetch } from "./mock";

const BASE = "/api";
const USE_MOCK = !import.meta.env.VITE_API_REAL;

export class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

async function request<T>(
  method: string,
  path: string,
  body?: unknown,
): Promise<T> {
  if (USE_MOCK) {
    const result = mockFetch(`${BASE}${path}`, body);
    if (result !== undefined) {
      await new Promise((r) => setTimeout(r, 80 + Math.random() * 120));
      return result as T;
    }
    throw new ApiError(404, "Mock: route not found");
  }

  const res = await fetch(`${BASE}${path}`, {
    method,
    headers: body ? { "Content-Type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
    credentials: "same-origin",
  });
  if (!res.ok) {
    const text = await res.text().catch(() => res.statusText);
    let msg = text;
    try { msg = JSON.parse(text).error || msg; } catch { /* plain text */ }
    throw new ApiError(res.status, msg);
  }
  if (res.status === 204) return undefined as T;
  return res.json();
}

export const api = {
  get: <T>(path: string) => request<T>("GET", path),
  post: <T>(path: string, body?: unknown) => request<T>("POST", path, body),
  put: <T>(path: string, body?: unknown) => request<T>("PUT", path, body),
  patch: <T>(path: string, body?: unknown) => request<T>("PATCH", path, body),
  delete: <T>(path: string) => request<T>("DELETE", path),
};

export async function downloadBlob(
  path: string,
  body: unknown,
  fallbackFilename: string,
): Promise<void> {
  const res = await fetch(`${BASE}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
    credentials: "same-origin",
  });
  if (!res.ok) {
    const text = await res.text().catch(() => res.statusText);
    let msg = text;
    try { msg = JSON.parse(text).error || msg; } catch { /* plain text */ }
    throw new ApiError(res.status, msg);
  }
  const blob = await res.blob();
  const disposition = res.headers.get("content-disposition");
  let filename = fallbackFilename;
  if (disposition) {
    const match = disposition.match(/filename\*?=(?:UTF-8'')?([^;\s]+)/i);
    if (match) filename = decodeURIComponent(match[1]);
  }
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}
