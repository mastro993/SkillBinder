import { Folder01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Link, useRouterState } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import {
  SidebarMenuSkeleton,
  SidebarMenuBadge,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
  useSidebar,
} from "@/components/ui/sidebar";
import { libraryListQuery } from "../hooks/queries";

export function SidebarFolderList() {
  const library = useQuery(libraryListQuery);
  const path = useRouterState({ select: (state) => state.location.pathname });
  const { setOpenMobile } = useSidebar();
  const folders = library.data?.folders ?? [];
  const counts = new Map<string, number>();
  for (const skill of library.data?.skills ?? []) {
    if (skill.folderId)
      counts.set(skill.folderId, (counts.get(skill.folderId) ?? 0) + 1);
  }

  if (library.isPending) {
    return (
      <SidebarMenuSub inset="left" aria-label="Folders" aria-busy="true">
        {[0, 1].map((index) => (
          <SidebarMenuSubItem key={index}>
            <SidebarMenuSkeleton showIcon />
          </SidebarMenuSubItem>
        ))}
      </SidebarMenuSub>
    );
  }
  if (folders.length === 0) return null;

  return (
    <SidebarMenuSub inset="left" aria-label="Folders">
      {[...folders]
        .sort((a, b) => a.name.localeCompare(b.name))
        .map((folder) => {
          const active = path === `/library/${folder.id}`;
          const count = counts.get(folder.id) ?? 0;
          return (
            <SidebarMenuSubItem key={folder.id}>
              <SidebarMenuSubButton
                render={
                  <Link
                    to="/library/$folderId"
                    params={{ folderId: folder.id }}
                    activeOptions={{ exact: true }}
                    title={folder.name}
                    aria-label={`${folder.name}, ${count} ${count === 1 ? "skill" : "skills"}`}
                    onClick={() => setOpenMobile(false)}
                  />
                }
                isActive={active}
                aria-current={active ? "page" : undefined}
              >
                <HugeiconsIcon icon={Folder01Icon} aria-hidden="true" />
                <span className="min-w-0 truncate">{folder.name}</span>
                <SidebarMenuBadge className="relative ml-auto shrink-0">
                  {count}
                </SidebarMenuBadge>
              </SidebarMenuSubButton>
            </SidebarMenuSubItem>
          );
        })}
    </SidebarMenuSub>
  );
}
