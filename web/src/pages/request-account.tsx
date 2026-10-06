import { useState, type FormEvent } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Card, CardContent } from "@/components/ui/card";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Loader2, CheckCircle2 } from "lucide-react";

export function RequestAccountPage() {
  const [contact, setContact] = useState("");
  const [unit, setUnit] = useState("");
  const [message, setMessage] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [sent, setSent] = useState(false);

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    if (!contact.trim()) {
      setError("Вкажіть контактні дані");
      return;
    }
    setError(null);
    setSubmitting(true);
    try {
      const res = await fetch("/api/auth/request-account", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          contact: contact.trim(),
          unit: unit.trim() || null,
          message: message.trim() || null,
        }),
      });
      if (!res.ok) {
        const text = await res.text().catch(() => res.statusText);
        throw new Error(text || `HTTP ${res.status}`);
      }
      setSent(true);
    } catch {
      setError("Не вдалося надіслати запит. Спробуйте пізніше.");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div
      className="flex min-h-screen items-center justify-center px-4"
      style={{
        background:
          "radial-gradient(ellipse 80% 60% at 50% 0%, rgba(201,168,76,0.04) 0%, transparent 70%), #0d0f0a",
      }}
    >
      <Card className="w-full max-w-sm">
        <CardContent className="flex flex-col gap-6 p-8">
          <div className="flex flex-col items-center gap-4">
            <img
              src="/emblem.png"
              alt="Вишкіл"
              className="h-32 w-32 object-contain sm:h-28 sm:w-28"
            />
            <div className="text-center">
              <h2
                style={{
                  margin: 0,
                  fontSize: "22px",
                  background:
                    "linear-gradient(135deg, #e8e4d8 0%, #c9a84c 100%)",
                  WebkitBackgroundClip: "text",
                  WebkitTextFillColor: "transparent",
                  backgroundClip: "text",
                }}
              >
                Вишкіл
              </h2>
              <p
                style={{
                  fontFamily: "var(--font-heading)",
                  fontSize: "10px",
                  letterSpacing: "0.25em",
                  textTransform: "uppercase",
                  color: "#8a8577",
                  marginTop: "4px",
                }}
              >
                Запит облікового запису
              </p>
            </div>
          </div>

          {sent ? (
            <div className="flex flex-col items-center gap-3 py-4">
              <CheckCircle2 className="h-10 w-10 text-primary" />
              <p className="text-center text-sm" style={{ color: "#8a8577" }}>
                Запит надіслано. Адміністратор зв'яжеться з вами.
              </p>
              <a
                href="/login"
                className="mt-2 text-sm"
                style={{ color: "var(--primary)", textDecoration: "none" }}
              >
                ← Повернутися до входу
              </a>
            </div>
          ) : (
            <form onSubmit={handleSubmit} className="flex flex-col gap-4">
              {error && (
                <Alert variant="destructive">
                  <AlertDescription>{error}</AlertDescription>
                </Alert>
              )}
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="contact">
                  Контактні дані (телефон, позивний) *
                </Label>
                <Input
                  id="contact"
                  value={contact}
                  onChange={(e) => setContact(e.target.value)}
                  placeholder="Телефон або позивний"
                  required
                />
              </div>
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="unit">Частина (необов'язково)</Label>
                <Input
                  id="unit"
                  value={unit}
                  onChange={(e) => setUnit(e.target.value)}
                  placeholder="Назва чи номер частини"
                />
              </div>
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="message">Коментар (необов'язково)</Label>
                <Textarea
                  id="message"
                  rows={3}
                  value={message}
                  onChange={(e) => setMessage(e.target.value)}
                  placeholder="Додаткова інформація"
                />
              </div>
              <Button type="submit" disabled={submitting} className="mt-2 w-full">
                {submitting && (
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                )}
                Надіслати запит
              </Button>
              <p
                className="text-center"
                style={{ fontSize: "12px", color: "#8a8577", margin: 0 }}
              >
                <a
                  href="/login"
                  style={{ color: "var(--primary)", textDecoration: "none" }}
                >
                  ← Повернутися до входу
                </a>
              </p>
            </form>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
