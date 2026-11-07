import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import { Plus, X } from "lucide-react";
import { api } from "@/api/client";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { DateInputUa } from "@/components/ui/date-input-ua";
import { fromIso } from "@/lib/date-ua";

/**
 * Особи події (модуль `personnel`, `.claude/decisions/personnel-named-persons.md`).
 * Рендериться ТІЛЬКИ коли `useFeatures().personnel === true` — вимкнений модуль не лишає в інтерфейсі й сліду.
 *
 * Правила приватності: значення полів ніколи не потрапляють в URL, localStorage, консоль чи тексти помилок/тостів;
 * дані живуть лише в стані компонента й зникають із ним.
 */
export interface PersonRow {
  id: number;
  rank: string | null;
  last_name: string;
  first_name: string;
  birth_date: string | null;
}

const RANKS = [
  "солдат", "старший солдат", "молодший сержант", "сержант", "старший сержант", "головний сержант",
  "штаб-сержант", "майстер-сержант", "старший майстер-сержант", "головний майстер-сержант",
  "молодший лейтенант", "лейтенант", "старший лейтенант", "капітан", "майор", "підполковник", "полковник",
];

const EMPTY = { rank: "", last_name: "", first_name: "", birth_date: "" };

export function EventPersons({ eventId, count }: { eventId: number; count: number }) {
  const [persons, setPersons] = useState<PersonRow[] | null>(null);
  const [adding, setAdding] = useState(false);
  const [form, setForm] = useState(EMPTY);
  const [saving, setSaving] = useState(false);

  const load = useCallback(() => {
    api
      .get<PersonRow[]>(`/data/events/${eventId}/persons`)
      .then(setPersons)
      .catch(() => {
        setPersons([]);
        toast.error("Не вдалося завантажити список осіб");
      });
  }, [eventId]);

  useEffect(() => {
    load();
    return () => setPersons(null);
  }, [load]);

  const full = count > 0 && persons !== null && persons.length >= count;
  const canSave = form.last_name.trim() !== "" && form.first_name.trim() !== "" && !saving;

  async function add() {
    if (!canSave) return;
    setSaving(true);
    try {
      await api.post(`/data/events/${eventId}/persons`, {
        rank: form.rank.trim() || null,
        last_name: form.last_name.trim(),
        first_name: form.first_name.trim(),
        birth_date: form.birth_date || null,
      });
      setForm(EMPTY);
      setAdding(false);
      load();
    } catch (e) {
      // Текст помилки сервера не містить значень полів (контракт модуля), але не покладаємося на це: показуємо його як є.
      toast.error(e instanceof Error ? e.message : "Не вдалося додати особу");
    } finally {
      setSaving(false);
    }
  }

  async function remove(personId: number) {
    try {
      await api.delete(`/data/events/${eventId}/persons/${personId}`);
      load();
    } catch {
      toast.error("Не вдалося зняти особу з події");
    }
  }

  return (
    <div className="ml-[82px] mt-1 flex flex-col gap-1.5 rounded-md border border-dashed p-2" data-testid="event-persons">
      <div className="flex items-center justify-between text-[11px] uppercase tracking-wider text-muted-foreground">
        <span>
          Особи: {persons ? persons.length : "…"} з {count}
        </span>
        {!adding && (
          <Button
            variant="ghost"
            size="sm"
            className="h-6 gap-1 px-1.5 text-[11px]"
            disabled={full}
            title={full ? "Осіб не може бути більше, ніж к-ть у події" : "Додати особу"}
            onClick={() => setAdding(true)}
          >
            <Plus className="h-3 w-3" />
            Додати
          </Button>
        )}
      </div>

      {persons && persons.length === 0 && !adding && (
        <div className="text-xs text-muted-foreground">Поіменний список не заповнено</div>
      )}

      {persons?.map((p) => (
        <div key={p.id} className="group/p flex items-center gap-2 text-sm" data-testid="event-person">
          <span className="w-[130px] shrink-0 truncate text-xs text-muted-foreground">{p.rank ?? "—"}</span>
          <span className="flex-1 truncate">
            {p.last_name} {p.first_name}
          </span>
          <span className="shrink-0 text-xs tabular-nums text-muted-foreground">{p.birth_date ? fromIso(p.birth_date) : ""}</span>
          <Button
            variant="ghost"
            size="icon"
            className="h-5 w-5 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
            title="Зняти з події"
            onClick={() => remove(p.id)}
          >
            <X className="h-3 w-3" />
          </Button>
        </div>
      ))}

      {adding && (
        <div className="flex flex-col gap-1.5">
          <div className="grid grid-cols-2 gap-1.5">
            <Input
              list={`ranks-${eventId}`}
              placeholder="Звання"
              value={form.rank}
              onChange={(e) => setForm({ ...form, rank: e.target.value })}
              autoComplete="off"
            />
            <datalist id={`ranks-${eventId}`}>
              {RANKS.map((r) => (
                <option key={r} value={r} />
              ))}
            </datalist>
            <DateInputUa value={form.birth_date} onChange={(iso) => setForm({ ...form, birth_date: iso })} placeholder="Дата народження" />
            <Input
              placeholder="Прізвище"
              value={form.last_name}
              onChange={(e) => setForm({ ...form, last_name: e.target.value })}
              autoComplete="off"
            />
            <Input
              placeholder="Ім'я"
              value={form.first_name}
              onChange={(e) => setForm({ ...form, first_name: e.target.value })}
              autoComplete="off"
            />
          </div>
          <div className="flex gap-2">
            <Button size="sm" className="h-7" disabled={!canSave} onClick={add}>
              {saving ? "Зберігаю…" : "Зберегти"}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              className="h-7"
              onClick={() => {
                setAdding(false);
                setForm(EMPTY);
              }}
            >
              Скасувати
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
