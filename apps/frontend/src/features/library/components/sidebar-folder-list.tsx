import { Folder01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Link, useRouterState } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import {
  SidebarMenuSkeleton,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
  useSidebar,
} from "@/components/ui/sidebar";
import { libraryListQuery } from "../hooks/queries";

/** Flat folders stay visible beneath Library so organization is clear on every page. */
export function SidebarFolderList() {
  const library = useQuery(libraryListQuery);
  const path = useRouterState({ select: (state) => state.location.pathname });
  const { setOpenMobile } = useSidebar();
  const folders = library.data?.folders ?? [];

  if (library.isPending) {
    return (
      <SidebarMenuSub aria-label="Folders" aria-busy="true">
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
    <SidebarMenuSub aria-label="Folders">
      {[...folders]
        .sort((a, b) => a.name.localeCompare(b.name))
        .map((folder) => {
          const active = path === `/library/${folder.id}`;
          return (
            <SidebarMenuSubItem key={folder.id}>
              <SidebarMenuSubButton
                render={
                  <Link
                    to="/library/$folderId"
                    params={{ folderId: folder.id }}
                    activeOptions={{ exact: true }}
                    title={folder.name}
                    onClick={() => setOpenMobile(false)}
                  />
                }
                isActive={active}
                aria-current={active ? "page" : undefined}
              >
                <HugeiconsIcon icon={Folder01Icon} aria-hidden="true" />
                <span className="truncate">{folder.name}</span>
              </SidebarMenuSubButton>
            </SidebarMenuSubItem>
          );
        })}
    </SidebarMenuSub>
  );
}
