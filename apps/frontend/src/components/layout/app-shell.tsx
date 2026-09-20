import type { PropsWithChildren } from "react";
import { Link, useRouterState } from "@tanstack/react-router";
import { Compass, BookOpen, Settings, Sparkles } from "lucide-react";
import { ScrollArea } from "@/components/ui/scroll-area";

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
            className={path === "/discovery" ? "nav-link active" : "nav-link"}
            to="/discovery"
          >
            <Compass size={18} /> Discovery
          </Link>
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
      <ScrollArea className="main-content">
        <main className="main-scroll-content">{children}</main>
      </ScrollArea>
    </div>
  );
}
