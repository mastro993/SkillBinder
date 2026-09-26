import type { PropsWithChildren } from "react";
import { Link, useRouterState } from "@tanstack/react-router";
import { HugeiconsIcon } from "@hugeicons/react";
import {
  BookOpen02Icon,
  CompassIcon,
  Link01Icon,
  Settings02Icon,
  SparklesIcon,
} from "@hugeicons/core-free-icons";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ScanActivity } from "@/features/discovery/components/scan-activity";

const navItems = [
  { to: "/discovery", label: "Discovery", icon: CompassIcon, activity: true },
  { to: "/library", label: "Library", icon: BookOpen02Icon, activity: false },
  { to: "/bindings", label: "Bindings", icon: Link01Icon, activity: false },
  { to: "/settings", label: "Settings", icon: Settings02Icon, activity: false },
];

const navLinkBase =
  "flex items-center gap-2.5 rounded-lg px-2.5 py-2.5 text-sm font-bold max-md:justify-center";

export function AppShell({ children }: PropsWithChildren) {
  const path = useRouterState({ select: (state) => state.location.pathname });
  return (
    <div className="grid h-screen grid-cols-[232px_minmax(0,1fr)] max-md:grid-cols-[72px_minmax(0,1fr)]">
      <aside className="flex flex-col overflow-hidden border-r border-sidebar-border bg-sidebar px-4 pt-7 pb-5">
        <div className="flex items-center gap-2.5 px-2 pb-7 text-lg font-extrabold tracking-tight">
          <span className="grid size-8 place-items-center rounded-lg bg-primary text-primary-foreground">
            <HugeiconsIcon icon={SparklesIcon} size={18} aria-hidden="true" />
          </span>
          <span className="max-md:hidden">SkillBinder</span>
        </div>
        <nav aria-label="Main navigation" className="grid gap-1.5">
          {navItems.map(({ to, label, icon, activity }) => (
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
              <HugeiconsIcon icon={icon} size={18} aria-hidden="true" />
              <span className="max-md:sr-only">{label}</span>
              {activity ? <ScanActivity /> : null}
            </Link>
          ))}
        </nav>
        <p className="mt-auto px-2 text-xs text-muted-foreground max-md:hidden">
          Local-first · No account
        </p>
      </aside>
      <ScrollArea className="min-h-0 min-w-0">
        <main className="min-w-0">{children}</main>
      </ScrollArea>
    </div>
  );
}
