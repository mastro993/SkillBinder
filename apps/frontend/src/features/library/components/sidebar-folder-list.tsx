import { Link, useRouterState } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { libraryListQuery } from "../hooks/queries";

/** Sidebar list of created folders. Each one opens that folder's own library page. */
export function SidebarFolderList() {
  const library = useQuery(libraryListQuery);
  const path = useRouterState({ select: (state) => state.location.pathname });
  const folders = library.data?.folders ?? [];
  if (folders.length === 0) return null;
  return (
    <ul aria-label="Folders" className="grid gap-1 pl-3 max-md:hidden">
      {[...folders]
        .sort((a, b) => a.name.localeCompare(b.name))
        .map((folder) => {
          const to = `/library/${folder.id}`;
          return (
            <li key={folder.id}>
              <Link
                to="/library/$folderId"
                params={{ folderId: folder.id }}
                aria-current={path === to ? "page" : undefined}
                className={
                  path === to
                    ? "flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm font-bold bg-sidebar-accent text-sidebar-accent-foreground"
                    : "flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm font-bold text-sidebar-foreground hover:bg-sidebar-accent/60 hover:text-sidebar-accent-foreground"
                }
              >
                <span className="truncate">{folder.name}</span>
              </Link>
            </li>
          );
        })}
    </ul>
  );
}
