import { useLocation } from "react-router-dom";
import { useRef, useEffect, type ReactNode } from "react";

export function PageTransition({ children }: { children: ReactNode }) {
  const location = useLocation();
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    el.classList.remove("page-transition");
    void el.offsetWidth;
    el.classList.add("page-transition");
  }, [location.pathname]);

  return (
    <div ref={ref} className="page-transition">
      {children}
    </div>
  );
}
