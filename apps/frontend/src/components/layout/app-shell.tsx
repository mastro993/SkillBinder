import type { PropsWithChildren, ReactNode } from "react";
import {
  BookOpen01Icon,
  Compass01Icon,
  GitBranchIcon,
  Settings01Icon,
  SparklesIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react";
import { Link, useRouterState } from "@tanstack/react-router";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
  SidebarSeparator,
  SidebarTrigger,
  useSidebar,
} from "@/components/ui/sidebar";
import { SidebarFolderList } from "@/features/library/components/sidebar-folder-list";
import { ScanActivity } from "@/features/discovery/components/scan-activity";
import { GitSyncNavStatus } from "@/features/git-sync/components/git-sync-nav-status";

interface NavItem {
  to: string;
  label: string;
  icon: IconSvgElement;
}

const discovery: NavItem = {
  to: "/discovery",
  label: "Discovery",
  icon: Compass01Icon,
};
const library: NavItem = {
  to: "/library",
  label: "Library",
  icon: BookOpen01Icon,
};
const footerItems: NavItem[] = [
  { to: "/git", label: "Sync", icon: GitBranchIcon },
  { to: "/settings", label: "Settings", icon: Settings01Icon },
];

function AppSidebar() {
  const path = useRouterState({ select: (state) => state.location.pathname });
  const { setOpenMobile } = useSidebar();

  function navItem({ to, label, icon }: NavItem, children?: ReactNode) {
    return (
      <SidebarMenuItem key={to}>
        <SidebarMenuButton
          render={
            <Link
              to={to}
              activeOptions={{ exact: true }}
              onClick={() => setOpenMobile(false)}
            />
          }
          isActive={path === to}
          aria-current={path === to ? "page" : undefined}
        >
          <HugeiconsIcon icon={icon} aria-hidden="true" />
          <span>{label}</span>
          {to === "/discovery" ? <ScanActivity /> : null}
          {to === "/git" ? <GitSyncNavStatus /> : null}
        </SidebarMenuButton>
        {children}
      </SidebarMenuItem>
    );
  }

  return (
    <Sidebar>
      <SidebarHeader>
        <div className="flex items-center gap-2.5 p-2 text-lg font-extrabold tracking-tight">
          <span className="grid size-8 place-items-center rounded-lg bg-primary text-primary-foreground">
            <HugeiconsIcon icon={SparklesIcon} size={18} aria-hidden="true" />
          </span>
          <span>SkillBinder</span>
        </div>
      </SidebarHeader>
      <SidebarContent>
        <nav aria-label="Main navigation">
          <SidebarGroup>
            <SidebarMenu>{navItem(discovery)}</SidebarMenu>
          </SidebarGroup>
          <SidebarSeparator />
          <SidebarGroup>
            <SidebarMenu>{navItem(library, <SidebarFolderList />)}</SidebarMenu>
          </SidebarGroup>
        </nav>
      </SidebarContent>
      <SidebarFooter>
        <nav aria-label="Secondary navigation">
          <SidebarMenu>{footerItems.map((item) => navItem(item))}</SidebarMenu>
        </nav>
      </SidebarFooter>
    </Sidebar>
  );
}

export function AppShell({ children }: PropsWithChildren) {
  return (
    <SidebarProvider open className="h-screen min-h-0">
      <AppSidebar />
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        <div className="md:hidden">
          <SidebarTrigger aria-label="Open navigation" />
        </div>
        <ScrollArea className="min-h-0 min-w-0 flex-1">
          <main className="min-w-0">{children}</main>
        </ScrollArea>
      </div>
    </SidebarProvider>
  );
}
