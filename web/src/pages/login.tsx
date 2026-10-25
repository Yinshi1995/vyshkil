import { useEffect, useState, type FormEvent } from "react";
import { useNavigate, Link, useSearchParams } from "react-router-dom";
import { useAuth } from "@/context/auth";
import { useAuthMethods } from "@/lib/auth-methods";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent } from "@/components/ui/card";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Fingerprint, Loader2 } from "lucide-react";

/** Коди з `/login?sso_error=` — див. `SsoError::code` у server/src/oidc.rs. */
const SSO_ERRORS: Record<string, string> = {
  disabled: "Вхід через ІКС не налаштовано на сервері.",
  unavailable: "ІКС тимчасово недоступний. Спробуйте пізніше.",
  denied: "Вхід через ІКС скасовано.",
  state: "Сеанс входу застарів. Натисніть «Увійти через ІКС» ще раз.",
  token: "ІКС повернув некоректну відповідь. Спробуйте ще раз.",
  forbidden: "Ваш профіль ІКС не має доступу до Вишколу. Зверніться до адміністратора ІКС.",
  pending: "Обліковий запис створено, чекає адміністратора.",
  inactive: "Обліковий запис неактивний — чекає адміністратора або деактивований.",
  conflict: "Цей логін уже прив'язано до іншого профілю ІКС. Зверніться до адміністратора.",
  internal: "Помилка сервера. Спробуйте пізніше.",
};

export function LoginPage() {
  const { login } = useAuth();
  const navigate = useNavigate();
  const methods = useAuthMethods();
  const [searchParams, setSearchParams] = useSearchParams();
  // Помилку з колбеку показуємо один раз і прибираємо з URL (щоб не висіла після оновлення).
  const [ssoError] = useState<string | null>(() => searchParams.get("sso_error"));
  const [ssoRedirecting, setSsoRedirecting] = useState(false);
  const [loginVal, setLoginVal] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    if (!searchParams.has("sso_error")) return;
    const next = new URLSearchParams(searchParams);
    next.delete("sso_error");
    setSearchParams(next, { replace: true });
  }, [searchParams, setSearchParams]);

  function handleSso() {
    setSsoRedirecting(true);
    // Повна навігація, не fetch: сервер редиректить на ІКС і назад.
    window.location.href = "/api/auth/oidc/login";
  }

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

          {ssoError && (
            <Alert variant={ssoError === "pending" ? "default" : "destructive"}>
              <AlertDescription>{SSO_ERRORS[ssoError] ?? "Не вдалося увійти через ІКС."}</AlertDescription>
            </Alert>
          )}

          {!methods ? (
            <div className="flex justify-center py-2">
              <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
            </div>
          ) : (
            <>
              {methods.sso && (
                <div className="flex flex-col gap-2">
                  <Button type="button" onClick={handleSso} disabled={ssoRedirecting} className="w-full" size="lg">
                    {ssoRedirecting ? (
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    ) : (
                      <Fingerprint className="mr-2 h-4 w-4" />
                    )}
                    Увійти через ІКС
                  </Button>
                  <p className="text-center" style={{ fontSize: "12px", color: "#8a8577", margin: 0 }}>
                    Ключ доступу (passkey) — один профіль для всіх застосунків
                  </p>
                </div>
              )}

              {!methods.sso && !methods.password && (
                <Alert>
                  <AlertDescription>Вхід не налаштовано. Зверніться до адміністратора.</AlertDescription>
                </Alert>
              )}

              {methods.password && (
                <>
                  {methods.sso && <div className="eyebrow" style={{ margin: "0" }}>або паролем</div>}
                  {renderPasswordForm(methods.sso)}
                </>
              )}
            </>
          )}
        </CardContent>
      </Card>
    </div>
  );

  // Аварійний вхід за паролем — лише коли сервер запущено з PASSWORD_LOGIN=1.
  // Викликається як функція, а не як <Компонент/>: інакше React перемонтовував би форму на
  // кожне натискання клавіші (новий тип компонента на кожен рендер) і поле втрачало б фокус.
  function renderPasswordForm(secondary: boolean) {
    return (
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
            <Button
              type="submit"
              disabled={submitting}
              variant={secondary ? "outline" : "default"}
              className="mt-2 w-full"
            >
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
    );
  }
}
