import { useRef, useState } from "react";
import { Folder01Icon, PlusSignIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Link, useRouterState } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import {
  SidebarGroup,
  SidebarGroupAction,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarMenuSkeleton,
  SidebarMenuBadge,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
  useSidebar,
} from "@/components/ui/sidebar";
import { FolderDialog } from "./folder-dialog";
import { libraryListQuery } from "../hooks/queries";

export function SidebarFolderList() {
  const [dialogOpen, setDialogOpen] = useState(false);
  const addButtonRef = useRef<HTMLButtonElement>(null);
  const library = useQuery(libraryListQuery);
  const path = useRouterState({ select: (state) => state.location.pathname });
  const { setOpenMobile } = useSidebar();
  const folders = library.data?.folders ?? [];
  const counts = new Map<string, number>();
  for (const skill of library.data?.skills ?? []) {
    if (skill.folderId)
      counts.set(skill.folderId, (counts.get(skill.folderId) ?? 0) + 1);
  }

  return (
    <SidebarGroup>
      <SidebarGroupLabel>Folders</SidebarGroupLabel>
      <SidebarGroupAction
        ref={addButtonRef}
        type="button"
        aria-label="New folder"
        title="New folder"
        onClick={() => setDialogOpen(true)}
      >
        <HugeiconsIcon icon={PlusSignIcon} aria-hidden="true" />
      </SidebarGroupAction>
      <SidebarGroupContent>
        <SidebarMenuSub
          inset="group"
          aria-label="Folders"
          aria-busy={library.isPending || undefined}
        >
          {library.isPending
            ? [0, 1].map((index) => (
                <SidebarMenuSubItem key={index}>
                  <SidebarMenuSkeleton showIcon />
                </SidebarMenuSubItem>
              ))
            : [...folders]
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
      </SidebarGroupContent>
      <FolderDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        finalFocus={addButtonRef}
      />
    </SidebarGroup>
  );
}
