import { lazy, Suspense } from "react";
import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { AuthProvider, useAuth } from "@/context/auth";
import { ThemeProvider } from "@/context/theme";
import { AppLayout } from "@/components/layout/app-layout";
import { ErrorBoundary } from "@/components/error-boundary";
import { Skeleton } from "@/components/ui/skeleton";
import { Toaster } from "@/components/ui/sonner";
import type { ReactNode } from "react";

const LoginPage = lazy(() => import("@/pages/login").then(m => ({ default: m.LoginPage })));
const ChangePasswordPage = lazy(() => import("@/pages/change-password").then(m => ({ default: m.ChangePasswordPage })));
const RequestAccountPage = lazy(() => import("@/pages/request-account").then(m => ({ default: m.RequestAccountPage })));
const HomePage = lazy(() => import("@/pages/home").then(m => ({ default: m.HomePage })));
const OrgDetailPage = lazy(() => import("@/pages/org-detail").then(m => ({ default: m.OrgDetailPage })));
const TrainingPage = lazy(() => import("@/pages/training").then(m => ({ default: m.TrainingPage })));
const DocumentsPage = lazy(() => import("@/pages/documents").then(m => ({ default: m.DocumentsPage })));
const DiscrepanciesPage = lazy(() => import("@/pages/discrepancies").then(m => ({ default: m.DiscrepanciesPage })));
const SettingsPage = lazy(() => import("@/pages/settings").then(m => ({ default: m.SettingsPage })));
const DataWorkspacePage = lazy(() => import("@/pages/data-workspace").then(m => ({ default: m.DataWorkspacePage })));
const ImportPage = lazy(() => import("@/pages/import").then(m => ({ default: m.ImportPage })));
const OrgsPage = lazy(() => import("@/pages/orgs").then(m => ({ default: m.OrgsPage })));

function PageLoader() {
  return (
    <div className="flex min-h-[50vh] items-center justify-center">
      <Skeleton className="h-8 w-48" />
    </div>
  );
}

function RequireAuth({ children }: { children: ReactNode }) {
  const { user, loading } = useAuth();
  if (loading) {
    return (
      <div className="flex min-h-screen items-center justify-center">
        <Skeleton className="h-8 w-48" />
      </div>
    );
  }
  if (!user) return <Navigate to="/login" replace />;
  return <>{children}</>;
}

function AppRoutes() {
  return (
    <Suspense fallback={<PageLoader />}>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route path="/change-password" element={<ChangePasswordPage />} />
        <Route path="/request-account" element={<RequestAccountPage />} />
        <Route
          path="/*"
          element={
            <RequireAuth>
              <AppLayout>
                <ErrorBoundary>
                  <Suspense fallback={<PageLoader />}>
                    <Routes>
                      <Route path="/" element={<HomePage />} />
                      <Route path="/orgs" element={<OrgsPage />} />
                      <Route path="/org/:id" element={<OrgDetailPage />} />
                      <Route path="/training" element={<TrainingPage />} />
                      <Route path="/data" element={<DataWorkspacePage />} />
                      <Route path="/documents" element={<DocumentsPage />} />
                      <Route path="/discrepancies" element={<DiscrepanciesPage />} />
                      <Route path="/settings" element={<SettingsPage />} />
                      <Route path="/import" element={<ImportPage />} />
                      <Route path="*" element={<p className="py-8 text-center">Сторінку не знайдено.</p>} />
                    </Routes>
                  </Suspense>
                </ErrorBoundary>
              </AppLayout>
            </RequireAuth>
          }
        />
      </Routes>
    </Suspense>
  );
}

export function App() {
  return (
    <ErrorBoundary>
      <BrowserRouter>
        <ThemeProvider>
          <AuthProvider>
            <AppRoutes />
            <Toaster position="bottom-right" richColors />
          </AuthProvider>
        </ThemeProvider>
      </BrowserRouter>
    </ErrorBoundary>
  );
}
