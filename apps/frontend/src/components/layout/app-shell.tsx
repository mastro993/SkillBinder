import type { PropsWithChildren } from "react";
import { Link, useRouterState } from "@tanstack/react-router";
import { BookOpen, Settings, Sparkles } from "lucide-react";

export function AppShell({ children }: PropsWithChildren) {
  const path = useRouterState({ select: (state) => state.location.pathname });
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark">
            <Sparkles size={18} />
          </span>
          SkillBinder
        </div>
        <nav aria-label="Main navigation">
          <Link
            className={path === "/library" ? "nav-link active" : "nav-link"}
            to="/library"
          >
            <BookOpen size={18} /> Library
          </Link>
          <Link
            className={path === "/settings" ? "nav-link active" : "nav-link"}
            to="/settings"
          >
            <Settings size={18} /> Settings
          </Link>
        </nav>
        <p className="sidebar-note">Local-first · No account</p>
      </aside>
      <main className="main-content">{children}</main>
    </div>
  );
}
