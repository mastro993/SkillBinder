import { useEffect, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { getDesktopClient } from "@/commands/client";
import {
  organizationNameSchema,
  type OrganizationNameForm,
} from "../types/organization";
import type {
  FolderView,
  OrganizationChange,
  OrganizationDeletePreviewResponse,
} from "@/types";

/**
 * Creates a folder, or renames and deletes an existing one. Folders hold skills
 * directly; there is no nesting.
 */
export function FolderDialog({
  folder,
  open,
  onOpenChange,
  onDeleted,
}: {
  folder?: FolderView;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onDeleted?: () => void;
}) {
  const queryClient = useQueryClient();
  const [preview, setPreview] =
    useState<OrganizationDeletePreviewResponse | null>(null);
  const [error, setError] = useState("");
  const {
    register,
    reset,
    handleSubmit,
    formState: { errors },
  } = useForm<OrganizationNameForm>({
    resolver: zodResolver(organizationNameSchema),
    defaultValues: { name: folder?.name ?? "" },
  });
  // `useForm` only reads `defaultValues` on mount, and this dialog stays mounted across
  // folders, so each open starts from the folder it is showing.
  useEffect(() => {
    if (open) reset({ name: folder?.name ?? "" });
  }, [open, folder?.id, folder?.name, reset]);
  const change = useMutation({
    mutationFn: async (request: {
      change: OrganizationChange;
      expectedRevision: string | null;
    }) => (await getDesktopClient()).libraryOrganizationChange(request),
    onSuccess: async (_result, request) => {
      await queryClient.invalidateQueries({ queryKey: ["library", "list"] });
      setPreview(null);
      setError("");
      onOpenChange(false);
      if (request.change.kind === "deleteFolder") onDeleted?.();
    },
    onError: (cause) => setError(cause.message),
  });
  const requestDelete = useMutation({
    mutationFn: async (target: FolderView) =>
      (await getDesktopClient()).libraryOrganizationPreviewDelete({
        entity: "folder",
        id: target.id,
      }),
    onSuccess: (result) => {
      setError("");
      setPreview(result);
    },
    onError: (cause) => setError(cause.message),
  });
  const save = (name: string) =>
    change.mutate({
      change: folder
        ? { kind: "updateFolder", id: folder.id, name }
        : { kind: "createFolder", name },
      expectedRevision: null,
    });
  return (
    <Dialog
      open={open}
      onOpenChange={(opened) => {
        if (!opened) {
          setPreview(null);
          setError("");
        }
        onOpenChange(opened);
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{folder ? "Edit folder" : "New folder"}</DialogTitle>
          <DialogDescription>
            {preview
              ? `Delete ${folder?.name}? ${preview.affectedSkills} skill${preview.affectedSkills === 1 ? "" : "s"} will move to Unfiled.`
              : ""}
          </DialogDescription>
        </DialogHeader>
        {preview ? null : (
          <label htmlFor="folder-name" className="grid gap-1 text-sm">
            Name
            <Input id="folder-name" {...register("name")} />
          </label>
        )}
        {errors.name && !preview ? (
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
          {folder && !preview ? (
            <Button
              variant="destructive"
              disabled={requestDelete.isPending}
              onClick={() => requestDelete.mutate(folder)}
            >
              Delete
            </Button>
          ) : null}
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          {preview ? (
            <Button
              variant="destructive"
              disabled={change.isPending}
              onClick={() =>
                change.mutate({
                  change: { kind: "deleteFolder", id: folder!.id },
                  expectedRevision: preview.organizationRevision,
                })
              }
            >
              Delete
            </Button>
          ) : (
            <Button
              disabled={change.isPending}
              onClick={() => void handleSubmit(({ name }) => save(name))()}
            >
              Save
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
