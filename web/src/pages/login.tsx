import { useState, type FormEvent } from "react";
import { useNavigate, Link } from "react-router-dom";
import { useAuth } from "@/context/auth";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent } from "@/components/ui/card";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Loader2 } from "lucide-react";

export function LoginPage() {
  const { login } = useAuth();
  const navigate = useNavigate();
  const [loginVal, setLoginVal] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      const res = await login({ login: loginVal, password });
      if (!res.success) {
        setError(res.error ?? "Невірний логін або пароль");
      } else if (!res.must_change_password) {
        navigate("/");
      }
    } catch {
      setError("Помилка з'єднання з сервером");
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
          {/* Brand */}
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
                  background: "linear-gradient(135deg, #e8e4d8 0%, #c9a84c 100%)",
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
                Облік заходів підготовки
              </p>
            </div>
          </div>

          <div className="eyebrow" style={{ margin: "0" }}>Авторизація</div>

          <form onSubmit={handleSubmit} className="flex flex-col gap-4">
            {error && (
              <Alert variant="destructive">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="login">Логін</Label>
              <Input
                id="login"
                value={loginVal}
                onChange={(e) => setLoginVal(e.target.value)}
                autoComplete="username"
                placeholder="admin"
                required
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="password">Пароль</Label>
              <Input
                id="password"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                autoComplete="current-password"
                placeholder="••••••••"
                required
              />
            </div>
            <Button type="submit" disabled={submitting} className="mt-2 w-full">
              {submitting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              Увійти
            </Button>
            <p className="text-center" style={{ fontSize: "12px", color: "#8a8577", margin: 0 }}>
              <Link
                to="/request-account"
                style={{ color: "var(--primary)", textDecoration: "none" }}
              >
                Запросити обліковий запис
              </Link>
            </p>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
