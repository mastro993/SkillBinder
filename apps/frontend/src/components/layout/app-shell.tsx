import type { PropsWithChildren } from "react";
import { Link, useRouterState } from "@tanstack/react-router";
import { BookOpen, Compass, GitBranch, Settings, Sparkles } from "lucide-react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ScanActivity } from "@/features/discovery/components/scan-activity";
import { GitSyncNavStatus } from "@/features/git-sync/components/git-sync-nav-status";

interface NavItem {
  to: string;
  label: string;
  icon: typeof Compass;
  activity: boolean;
}

const mainNavItems: NavItem[] = [
  { to: "/discovery", label: "Discovery", icon: Compass, activity: true },
  { to: "/library", label: "Library", icon: BookOpen, activity: false },
];

const footerNavItems: NavItem[] = [
  { to: "/git", label: "Git sync", icon: GitBranch, activity: false },
  { to: "/settings", label: "Settings", icon: Settings, activity: false },
];

const navLinkBase =
  "flex items-center gap-2.5 rounded-lg px-2.5 py-2.5 text-sm font-bold max-md:justify-center";

export function AppShell({ children }: PropsWithChildren) {
  const path = useRouterState({ select: (state) => state.location.pathname });
  function renderNavItem({ to, label, icon: Icon, activity }: NavItem) {
    return (
      <Link
        key={to}
        to={to}
        aria-current={path === to ? "page" : undefined}
        className={
          path === to
            ? `${navLinkBase} bg-sidebar-accent text-sidebar-accent-foreground`
            : `${navLinkBase} text-sidebar-foreground hover:bg-sidebar-accent/60 hover:text-sidebar-accent-foreground`
        }
      >
        <Icon size={18} aria-hidden="true" />
        <span className="max-md:hidden">{label}</span>
        {activity ? <ScanActivity /> : null}
        {to === "/git" ? <GitSyncNavStatus /> : null}
      </Link>
    );
  }
  return (
    <div className="grid h-screen grid-cols-[232px_minmax(0,1fr)] max-md:grid-cols-[72px_minmax(0,1fr)]">
      <aside className="flex flex-col overflow-hidden border-r border-sidebar-border bg-sidebar px-4 pt-7 pb-5">
        <div className="flex items-center gap-2.5 px-2 pb-7 text-lg font-extrabold tracking-tight">
          <span className="grid size-8 place-items-center rounded-lg bg-primary text-primary-foreground">
            <Sparkles size={18} aria-hidden="true" />
          </span>
          <span className="max-md:hidden">SkillBinder</span>
        </div>
        <nav aria-label="Main navigation" className="grid gap-1.5">
          {mainNavItems.map(renderNavItem)}
        </nav>
        <nav aria-label="Secondary navigation" className="mt-auto grid gap-1.5">
          {footerNavItems.map(renderNavItem)}
        </nav>
      </aside>
      <ScrollArea className="min-h-0 min-w-0">
        <main className="min-w-0">{children}</main>
      </ScrollArea>
    </div>
  );
}
