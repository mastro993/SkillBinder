import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { getDesktopClient } from "@/commands/client";
import type { LibraryListResponse } from "@/types";

/** Assigns the selected skills to one folder, or to Unfiled. */
export function AssignFolderDialog({
  data,
  skillIds,
  onOpenChange,
}: {
  data: LibraryListResponse;
  skillIds: string[];
  onOpenChange: (open: boolean) => void;
}) {
  const queryClient = useQueryClient();
  const [error, setError] = useState("");
  const soleSkill =
    skillIds.length === 1
      ? data.skills.find((skill) => skill.skillId === skillIds[0])
      : undefined;
  const [folderId, setFolderId] = useState<string | null | undefined>(
    soleSkill ? soleSkill.folderId : undefined,
  );
  const change = useMutation({
    mutationFn: async () => {
      await (
        await getDesktopClient()
      ).libraryOrganizationChange({
        change: {
          kind: "assign",
          skillIds,
          folderId: folderId ?? null,
          setFolder: folderId !== undefined,
          addTagIds: [],
          removeTagIds: [],
        },
        expectedRevision: null,
      });
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["library", "list"] });
      setError("");
      onOpenChange(false);
    },
    onError: (cause) => setError(cause.message),
  });
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) onOpenChange(false);
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>
            {skillIds.length === 1
              ? "Organize skill"
              : `Organize ${skillIds.length} skills`}
          </DialogTitle>
          <DialogDescription>
            {skillIds.length === 1
              ? "Move this skill into a folder."
              : "Move every selected skill into one folder."}
          </DialogDescription>
        </DialogHeader>
        <label htmlFor="assign-folder" className="grid gap-1 text-sm">
          Folder
          <select
            id="assign-folder"
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
            {folderId === undefined ? (
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
        {error ? (
          <p role="alert" className="text-sm text-destructive">
            {error}
          </p>
        ) : null}
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button disabled={change.isPending} onClick={() => change.mutate()}>
            Save
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
