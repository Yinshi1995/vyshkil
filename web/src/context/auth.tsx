import {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  type ReactNode,
} from "react";
import { useNavigate } from "react-router-dom";
import { api, ApiError } from "@/api/client";
import type { AuthUser, LoginRequest, LoginResponse, Actor } from "@/api/types";

interface AuthCtx {
  user: AuthUser | null;
  loading: boolean;
  actor: Actor | null;
  isAdmin: boolean;
  login: (req: LoginRequest) => Promise<LoginResponse>;
  logout: () => Promise<void>;
  switchActor: (orgId: number, role: string) => Promise<void>;
  refresh: () => Promise<void>;
}

const AuthContext = createContext<AuthCtx | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [loading, setLoading] = useState(true);
  const navigate = useNavigate();

  const refresh = useCallback(async () => {
    try {
      const u = await api.get<AuthUser>("/auth/me");
      setUser(u);
    } catch (e) {
      if (e instanceof ApiError && e.status === 401) {
        setUser(null);
      }
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const login = useCallback(
    async (req: LoginRequest): Promise<LoginResponse> => {
      const res = await api.post<LoginResponse>("/auth/login", req);
      if (res.success) {
        await refresh();
        if (res.must_change_password) {
          navigate("/change-password");
        }
      }
      return res;
    },
    [refresh, navigate],
  );

  const logout = useCallback(async () => {
    await api.post("/auth/logout");
    setUser(null);
    navigate("/login");
  }, [navigate]);

  const switchActor = useCallback(
    async (orgId: number, role: string) => {
      await api.post("/auth/switch-actor", { org_id: orgId, role });
      await refresh();
    },
    [refresh],
  );

  const actor = user?.actor ?? null;
  const isAdmin = actor?.role === "admin";

  return (
    <AuthContext.Provider
      value={{ user, loading, actor, isAdmin, login, logout, switchActor, refresh }}
    >
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth must be used within AuthProvider");
  return ctx;
}
