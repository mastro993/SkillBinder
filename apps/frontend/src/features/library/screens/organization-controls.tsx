import { forwardRef, useImperativeHandle, useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { getDesktopClient } from "@/commands/client";
import {
  organizationNameSchema,
  type OrganizationNameForm,
} from "../types/organization";
import type {
  FolderView,
  LibraryListResponse,
  OrganizationChange,
  OrganizationDeletePreviewResponse,
} from "@/types";

export type FolderScope =
  | { kind: "all" }
  | { kind: "unfiled" }
  | { kind: "folder"; id: string };
type Edit =
  | { kind: "folder"; folder?: FolderView; parentId: string | null }
  | { kind: "tag"; tagId?: string; name?: string }
  | { kind: "assign"; skillIds: string[] }
  | {
      kind: "delete";
      entity: "folder" | "tag";
      id: string;
      name: string;
      preview: OrganizationDeletePreviewResponse;
    };

export function includesFolder(
  folderId: string | null,
  selectedId: string,
  folders: FolderView[],
): boolean {
  let current = folderId;
  while (current) {
    if (current === selectedId) return true;
    current = folders.find((folder) => folder.id === current)?.parentId ?? null;
  }
  return false;
}

interface Props {
  data: LibraryListResponse;
  scope: FolderScope;
  setScope: (scope: FolderScope) => void;
  selectedIds: string[];
  clearSelection: () => void;
  activeTags: string[];
  setActiveTags: (ids: string[]) => void;
  matchAny: boolean;
  setMatchAny: (value: boolean) => void;
}

export interface OrganizationControlsHandle {
  organizeSkill: (id: string) => void;
}
export const OrganizationControls = forwardRef<
  OrganizationControlsHandle,
  Props
>(function OrganizationControls(
  {
    data,
    scope,
    setScope,
    selectedIds,
    clearSelection,
    activeTags,
    setActiveTags,
    matchAny,
    setMatchAny,
  },
  ref,
) {
  const queryClient = useQueryClient();
  const [edit, setEdit] = useState<Edit | null>(null);
  const {
    register,
    reset,
    handleSubmit,
    formState: { errors },
  } = useForm<OrganizationNameForm>({
    resolver: zodResolver(organizationNameSchema),
    defaultValues: { name: "" },
  });
  const [parentId, setParentId] = useState<string | null>(null);
  const [folderId, setFolderId] = useState<string | null | undefined>(null);
  const [tagAction, setTagAction] = useState<"add" | "remove">("add");
  const [tagIds, setTagIds] = useState<string[]>([]);
  const [error, setError] = useState("");
  const change = useMutation({
    mutationFn: async (request: {
      change: OrganizationChange;
      expectedRevision: string | null;
    }) => (await getDesktopClient()).libraryOrganizationChange(request),
    onSuccess: async (_result, request) => {
      if (
        request.change.kind === "deleteFolder" &&
        scope.kind === "folder" &&
        scope.id === request.change.id
      )
        setScope({ kind: "all" });
      if (request.change.kind === "deleteTag") {
        const deletedId = request.change.id;
        setActiveTags(activeTags.filter((id) => id !== deletedId));
      }
      await queryClient.invalidateQueries({ queryKey: ["library", "list"] });
      setEdit(null);
      setError("");
      clearSelection();
    },
    onError: (cause) => setError(cause.message),
  });
  const preview = useMutation({
    mutationFn: async (request: {
      entity: "folder" | "tag";
      id: string;
      name: string;
    }) => ({
      request,
      result: await (
        await getDesktopClient()
      ).libraryOrganizationPreviewDelete({
        entity: request.entity,
        id: request.id,
      }),
    }),
    onSuccess: ({ request, result }) => {
      setError("");
      setEdit({ kind: "delete", ...request, preview: result });
    },
    onError: (cause) => setError(cause.message),
  });
  const open = (next: Edit) => {
    setEdit(next);
    setError("");
    reset({
      name:
        next.kind === "folder"
          ? (next.folder?.name ?? "")
          : next.kind === "tag"
            ? (next.name ?? "")
            : "",
    });
    setParentId(
      next.kind === "folder" ? (next.folder?.parentId ?? next.parentId) : null,
    );
    if (next.kind === "assign") {
      const assigned = data.skills.filter((skill) =>
        next.skillIds.includes(skill.skillId),
      );
      setFolderId(assigned.length === 1 ? assigned[0].folderId : undefined);
      setTagAction("add");
      setTagIds(assigned.length === 1 ? assigned[0].tagIds : []);
    }
  };
  const submit = (name: string) => {
    if (!edit) return;
    let command: OrganizationChange;
    if (edit.kind === "folder") {
      command = edit.folder
        ? { kind: "updateFolder", id: edit.folder.id, name, parentId }
        : { kind: "createFolder", name, parentId };
    } else if (edit.kind === "tag") {
      command = edit.tagId
        ? { kind: "renameTag", id: edit.tagId, name }
        : { kind: "createTag", name };
    } else if (edit.kind === "assign") {
      const currentTags =
        edit.skillIds.length === 1
          ? (data.skills.find((skill) => skill.skillId === edit.skillIds[0])
              ?.tagIds ?? [])
          : [];
      command = {
        kind: "assign",
        skillIds: edit.skillIds,
        folderId: folderId ?? null,
        setFolder: folderId !== undefined,
        addTagIds: tagAction === "add" ? tagIds : [],
        removeTagIds:
          edit.skillIds.length === 1
            ? currentTags.filter((id) => !tagIds.includes(id))
            : tagAction === "remove"
              ? tagIds
              : [],
      };
    } else {
      command =
        edit.entity === "folder"
          ? { kind: "deleteFolder", id: edit.id }
          : { kind: "deleteTag", id: edit.id };
    }
    change.mutate({
      change: command,
      expectedRevision:
        edit.kind === "delete" ? edit.preview.organizationRevision : null,
    });
  };
  useImperativeHandle(ref, () => ({
    organizeSkill: (id: string) => open({ kind: "assign", skillIds: [id] }),
  }));
  const selectableParents = (folder?: FolderView) =>
    data.folders.filter(
      (item) =>
        !folder ||
        (item.id !== folder.id &&
          !includesFolder(item.id, folder.id, data.folders)),
    );
  const folderRows = (parent: string | null): React.ReactNode =>
    data.folders
      .filter((folder) => folder.parentId === parent)
      .sort((a, b) => a.name.localeCompare(b.name))
      .map((folder) => (
        <li key={folder.id}>
          <div className="flex items-center gap-1">
            <Button
              variant={
                scope.kind === "folder" && scope.id === folder.id
                  ? "secondary"
                  : "ghost"
              }
              size="sm"
              className="min-w-0 flex-1 justify-start"
              onClick={() => setScope({ kind: "folder", id: folder.id })}
            >
              {folder.name}
            </Button>
            <Button
              variant="ghost"
              size="xs"
              aria-label={`Edit ${folder.name}`}
              onClick={() =>
                open({ kind: "folder", folder, parentId: folder.parentId })
              }
            >
              Edit
            </Button>
          </div>
          <ul className="pl-3">{folderRows(folder.id)}</ul>
        </li>
      ));
  return (
    <>
      <aside
        className="grid content-start gap-5 border-r pr-5"
        aria-label="Organize library"
      >
        <div className="grid gap-1">
          <div className="flex items-center justify-between">
            <h2 className="text-sm font-medium">Folders</h2>
            <Button
              variant="ghost"
              size="xs"
              onClick={() =>
                open({
                  kind: "folder",
                  parentId: scope.kind === "folder" ? scope.id : null,
                })
              }
            >
              New
            </Button>
          </div>
          <Button
            variant={scope.kind === "all" ? "secondary" : "ghost"}
            size="sm"
            className="justify-start"
            onClick={() => setScope({ kind: "all" })}
          >
            All skills
          </Button>
          <Button
            variant={scope.kind === "unfiled" ? "secondary" : "ghost"}
            size="sm"
            className="justify-start"
            onClick={() => setScope({ kind: "unfiled" })}
          >
            Unfiled
          </Button>
          <ul className="grid gap-1">{folderRows(null)}</ul>
        </div>
        <div className="grid gap-2">
          <div className="flex items-center justify-between">
            <h2 className="text-sm font-medium">Tags</h2>
            <Button
              variant="ghost"
              size="xs"
              onClick={() => open({ kind: "tag" })}
            >
              New
            </Button>
          </div>
          {data.tags.length === 0 ? (
            <p className="text-xs text-muted-foreground">No tags yet.</p>
          ) : null}
          <ul className="grid gap-1">
            {data.tags.map((tag) => (
              <li key={tag.id} className="flex items-center gap-2">
                <Checkbox
                  aria-label={`Filter by ${tag.name}`}
                  checked={activeTags.includes(tag.id)}
                  onCheckedChange={(checked) =>
                    setActiveTags(
                      checked
                        ? [...activeTags, tag.id]
                        : activeTags.filter((id) => id !== tag.id),
                    )
                  }
                />
                <span className="min-w-0 flex-1 truncate text-sm">
                  {tag.name}
                </span>
                <Button
                  variant="ghost"
                  size="xs"
                  aria-label={`Edit tag ${tag.name}`}
                  onClick={() =>
                    open({ kind: "tag", tagId: tag.id, name: tag.name })
                  }
                >
                  Edit
                </Button>
              </li>
            ))}
          </ul>
          {activeTags.length > 1 ? (
            <Button
              variant="ghost"
              size="xs"
              className="justify-start"
              onClick={() => setMatchAny(!matchAny)}
            >
              {matchAny ? "Match any tag" : "Match all tags"}
            </Button>
          ) : null}
        </div>
        {selectedIds.length ? (
          <div className="grid gap-2 border-t pt-4">
            <p className="text-sm">{selectedIds.length} selected</p>
            <Button
              variant="outline"
              size="sm"
              onClick={() => open({ kind: "assign", skillIds: selectedIds })}
            >
              Organize selected
            </Button>
            <Button variant="ghost" size="sm" onClick={clearSelection}>
              Clear selection
            </Button>
          </div>
        ) : null}
        {error && !edit ? (
          <p role="alert" className="text-sm text-destructive">
            {error}
          </p>
        ) : null}
      </aside>
      <Dialog
        open={edit !== null}
        onOpenChange={(opened) => {
          if (!opened) setEdit(null);
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {edit?.kind === "folder"
                ? edit.folder
                  ? "Edit folder"
                  : "New folder"
                : edit?.kind === "tag"
                  ? edit.tagId
                    ? "Edit tag"
                    : "New tag"
                  : edit?.kind === "assign"
                    ? `Organize ${edit.skillIds.length} skill${edit.skillIds.length === 1 ? "" : "s"}`
                    : edit?.entity === "folder"
                      ? "Delete folder"
                      : "Delete tag"}
            </DialogTitle>
            {edit?.kind === "delete" ? (
              <DialogDescription>
                {edit.entity === "folder"
                  ? `Delete ${edit.name}? ${edit.preview.childFolders} child folder${edit.preview.childFolders === 1 ? "" : "s"} and ${edit.preview.affectedSkills} skill${edit.preview.affectedSkills === 1 ? "" : "s"} will move up one level.`
                  : `Delete ${edit.name}? It will be removed from ${edit.preview.affectedSkills} skill${edit.preview.affectedSkills === 1 ? "" : "s"}.`}
              </DialogDescription>
            ) : null}
          </DialogHeader>
          {edit?.kind === "folder" ? (
            <div className="grid gap-3">
              <label htmlFor="organization-name" className="grid gap-1 text-sm">
                Name
                <Input id="organization-name" {...register("name")} />
              </label>
              <label
                htmlFor="organization-parent"
                className="grid gap-1 text-sm"
              >
                Parent folder
                <select
                  id="organization-parent"
                  className="h-8 rounded-lg border border-input bg-background px-2 text-sm"
                  value={parentId ?? ""}
                  onChange={(event) => setParentId(event.target.value || null)}
                >
                  <option value="">No parent</option>
                  {selectableParents(edit.folder).map((folder) => (
                    <option key={folder.id} value={folder.id}>
                      {folder.name}
                    </option>
                  ))}
                </select>
              </label>
            </div>
          ) : null}
          {edit?.kind === "tag" ? (
            <label htmlFor="organization-name" className="grid gap-1 text-sm">
              Name
              <Input id="organization-name" {...register("name")} />
            </label>
          ) : null}
          {edit?.kind === "assign" ? (
            <div className="grid gap-3">
              <label
                htmlFor="organization-folder"
                className="grid gap-1 text-sm"
              >
                Folder
                <select
                  id="organization-folder"
                  className="h-8 rounded-lg border border-input bg-background px-2 text-sm"
                  value={folderId === undefined ? "__keep__" : (folderId ?? "")}
                  onChange={(event) =>
                    setFolderId(
                      event.target.value === "__keep__"
                        ? undefined
                        : event.target.value || null,
                    )
                  }
                >
                  {edit.skillIds.length > 1 ? (
                    <option value="__keep__">Keep current folders</option>
                  ) : null}
                  <option value="">Unfiled</option>
                  {data.folders.map((folder) => (
                    <option key={folder.id} value={folder.id}>
                      {folder.name}
                    </option>
                  ))}
                </select>
              </label>
              <fieldset className="grid gap-2">
                <legend className="mb-2 text-sm">
                  {edit.skillIds.length > 1 ? "Tags" : "Tags"}
                </legend>
                {edit.skillIds.length > 1 ? (
                  <div className="flex gap-2">
                    <Button
                      variant={tagAction === "add" ? "secondary" : "ghost"}
                      size="xs"
                      onClick={() => {
                        setTagAction("add");
                        setTagIds([]);
                      }}
                    >
                      Add
                    </Button>
                    <Button
                      variant={tagAction === "remove" ? "secondary" : "ghost"}
                      size="xs"
                      onClick={() => {
                        setTagAction("remove");
                        setTagIds([]);
                      }}
                    >
                      Remove
                    </Button>
                  </div>
                ) : null}
                {data.tags.map((tag) => (
                  <label
                    key={tag.id}
                    className="flex items-center gap-2 text-sm"
                  >
                    <Checkbox
                      checked={tagIds.includes(tag.id)}
                      onCheckedChange={(checked) =>
                        setTagIds(
                          checked
                            ? [...tagIds, tag.id]
                            : tagIds.filter((id) => id !== tag.id),
                        )
                      }
                    />
                    {tag.name}
                  </label>
                ))}
              </fieldset>
              {edit.skillIds.length > 1 ? (
                <p className="text-xs text-muted-foreground">
                  {tagAction === "add"
                    ? "Selected tags are added to every skill."
                    : "Selected tags are removed from every skill."}
                </p>
              ) : null}
            </div>
          ) : null}
          {errors.name && (edit?.kind === "folder" || edit?.kind === "tag") ? (
            <p role="alert" className="text-sm text-destructive">
              {errors.name.message}
            </p>
          ) : null}
          {error ? (
            <p role="alert" className="text-sm text-destructive">
              {error}
            </p>
          ) : null}
          <DialogFooter>
            {(edit?.kind === "folder" && edit.folder) ||
            (edit?.kind === "tag" && edit.tagId) ? (
              <Button
                variant="destructive"
                disabled={preview.isPending}
                onClick={() => {
                  if (edit?.kind === "folder" && edit.folder)
                    preview.mutate({
                      entity: "folder",
                      id: edit.folder.id,
                      name: edit.folder.name,
                    });
                  if (edit?.kind === "tag" && edit.tagId)
                    preview.mutate({
                      entity: "tag",
                      id: edit.tagId,
                      name: edit.name ?? "tag",
                    });
                }}
              >
                Delete
              </Button>
            ) : null}
            <Button variant="outline" onClick={() => setEdit(null)}>
              Cancel
            </Button>
            <Button
              variant={edit?.kind === "delete" ? "destructive" : "default"}
              disabled={change.isPending}
              onClick={() => {
                if (edit?.kind === "folder" || edit?.kind === "tag")
                  void handleSubmit(({ name }) => submit(name))();
                else submit("");
              }}
            >
              {edit?.kind === "delete" ? "Delete" : "Save"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
});
