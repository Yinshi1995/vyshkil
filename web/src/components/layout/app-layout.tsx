import type { ReactNode } from "react";
import { Header } from "./header";
import { IksRail } from "./iks-rail";
import { TooltipProvider } from "@/components/ui/tooltip";
import { LAB } from "@/lib/iks";

export function AppLayout({ children }: { children: ReactNode }) {
  return (
    <TooltipProvider>
      {LAB && <IksRail />}
      {/* Рейка ІКС фіксована зліва — вміст зсувається на її ширину (лише ≥1024 px). */}
      <div className={`flex min-h-screen flex-col${LAB ? " iks-shifted" : ""}`}>
        <Header />
        <main className="mx-auto w-full max-w-7xl flex-1 px-4 py-6">{children}</main>
      </div>
    </TooltipProvider>
  );
}
