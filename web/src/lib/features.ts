import { useEffect, useState } from "react";
import { api } from "@/api/client";

/**
 * Можливості, що вмикає сервер (`GET /api/features`).
 *
 * `personnel` — модуль "особи поіменно" (`.claude/decisions/personnel-named-persons.md`): ВИМКНЕНИЙ за
 * замовчуванням до юридичної перевірки. Доки він `false` (а також поки відповідь завантажується або
 * сервер недоступний), інтерфейс НЕ показує жодного елемента цього модуля й не робить жодних запитів до нього.
 */
export interface Features {
  personnel: boolean;
}

const OFF: Features = { personnel: false };

// Один запит на сесію вкладки. Помилка → "усе вимкнено" (безпечний дефолт), без повторів у циклі.
let cached: Promise<Features> | null = null;

function load(): Promise<Features> {
  if (!cached) {
    cached = api
      .get<Partial<Features>>("/features")
      .then((f) => ({ personnel: f?.personnel === true }))
      .catch(() => OFF);
  }
  return cached;
}

/** Завжди об'єкт: до відповіді сервера — усе вимкнено. */
export function useFeatures(): Features {
  const [features, setFeatures] = useState<Features>(OFF);
  useEffect(() => {
    let alive = true;
    load().then((f) => {
      if (alive) setFeatures(f);
    });
    return () => {
      alive = false;
    };
  }, []);
  return features;
}
