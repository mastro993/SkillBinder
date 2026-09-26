import { useQueries, useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { getDesktopClient } from "@/commands/client";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Skeleton } from "@/components/ui/skeleton";
import type { LibrarySkill } from "@/types";
import { payloadPath } from "../lib/conflicts";

export function SlugConflictDialog({
  slug,
  skills,
  keptSkillId,
  resolving,
  onKeepChange,
  onConfirm,
  onClose,
}: {
  slug: string;
  skills: Array<LibrarySkill>;
  keptSkillId: string;
  resolving: boolean;
  onKeepChange: (skillId: string) => void;
  onConfirm: () => void;
  onClose: () => void;
}) {
  const others = skills.length - 1;
  const [selectedPath, setSelectedPath] = useState("SKILL.md");
  const summaries = useQueries({
    queries: skills.map((skill) => ({
      queryKey: ["library", "skillPreview", skill.skillId, "SKILL.md"],
      queryFn: async () =>
        (await getDesktopClient()).librarySkillPreview({
          skillId: skill.skillId,
          path: null,
        }),
    })),
  });
  const selectedIndex = skills.findIndex(
    (skill) => skill.skillId === keptSkillId,
  );
  const selectedSummary = summaries[selectedIndex];
  const selectedFile = useQuery({
    queryKey: ["library", "skillPreview", keptSkillId, selectedPath],
    queryFn: async () =>
      (await getDesktopClient()).librarySkillPreview({
        skillId: keptSkillId,
        path: selectedPath,
      }),
    enabled: selectedPath !== "SKILL.md",
  });
  const preview = selectedPath === "SKILL.md" ? selectedSummary : selectedFile;
  return (
    <Dialog
      open
      modal
      onOpenChange={(open: boolean) => {
        if (!open && !resolving) onClose();
      }}
    >
      <DialogContent
        showCloseButton={false}
        className="max-h-[85vh] overflow-y-auto sm:max-w-4xl"
      >
        <DialogHeader>
          <p className="text-xs font-extrabold tracking-widest text-warning uppercase">
            Slug shared
          </p>
          <DialogTitle>Choose the copy to keep for {slug}</DialogTitle>
        </DialogHeader>
        <DialogDescription>
          {skills.length} skills are named <code>{slug}</code>. The chosen copy
          stays at <code>library/skills/{slug}</code>. The other{" "}
          {others === 1 ? "copy moves" : "copies move"} into the SkillBinder
          backup folder. Nothing is deleted from your agent folders, and nothing
          is committed until you Sync.
        </DialogDescription>
        <ul className="grid gap-2 sm:grid-cols-2">
          {skills.map((skill) => {
            const selected = skill.skillId === keptSkillId;
            const summary = summaries[skills.indexOf(skill)];
            return (
              <li key={skill.skillId}>
                <label
                  className={`flex cursor-pointer items-start gap-3 rounded-lg border px-3 py-2.5 text-sm transition-colors ${
                    selected
                      ? "border-primary bg-muted"
                      : "border-input hover:bg-muted/50"
                  }`}
                >
                  <input
                    type="radio"
                    name={`copy-of-${slug}`}
                    value={skill.skillId}
                    checked={selected}
                    disabled={resolving}
                    onChange={() => {
                      setSelectedPath("SKILL.md");
                      onKeepChange(skill.skillId);
                    }}
                    className="mt-1 accent-primary"
                  />
                  <span className="grid gap-1">
                    <span className="font-semibold">{payloadPath(skill)}</span>
                    <span className="text-xs text-muted-foreground">
                      {skill.fileCount} files · {skill.totalBytes} bytes
                    </span>
                    <span className="text-xs text-muted-foreground">
                      Last changed in library:{" "}
                      {summary?.data?.lastEditedAt != null
                        ? new Date(
                            summary.data.lastEditedAt * 1000,
                          ).toLocaleString()
                        : summary?.isPending
                          ? "Loading…"
                          : "Unavailable"}
                    </span>
                    <span className="text-xs text-muted-foreground">
                      {skill.description ?? "No description"}
                    </span>
                    {skill.sources.length ? (
                      <span className="break-words text-xs text-muted-foreground">
                        Sources:{" "}
                        {skill.sources
                          .map((source) => source.displayPath)
                          .join(", ")}
                      </span>
                    ) : null}
                  </span>
                </label>
              </li>
            );
          })}
        </ul>
        <section
          aria-label="Skill contents"
          className="grid gap-3 rounded-lg border p-3"
        >
          <div className="grid gap-1">
            <h3 className="font-semibold">
              Explore {skills[selectedIndex]?.slug}
            </h3>
            <p className="text-xs text-muted-foreground">
              {skills[selectedIndex] ? payloadPath(skills[selectedIndex]) : ""}
            </p>
          </div>
          {selectedSummary?.isError ? (
            <div className="flex items-center gap-2">
              <p role="alert" className="text-sm text-destructive">
                Could not load this copy.
              </p>
              <Button
                size="sm"
                variant="outline"
                onClick={() => selectedSummary.refetch()}
              >
                Retry
              </Button>
            </div>
          ) : selectedSummary?.isPending ? (
            <>
              <output className="sr-only">Loading skill contents</output>
              <div className="grid gap-2">
                <div className="flex max-h-24 flex-wrap gap-1">
                  <Skeleton className="h-7 w-20" />
                  <Skeleton className="h-7 w-24" />
                  <Skeleton className="h-7 w-16" />
                </div>
                <Skeleton className="h-48 w-full" />
              </div>
            </>
          ) : (
            <>
              <div className="flex max-h-24 flex-wrap gap-1 overflow-y-auto">
                {selectedSummary?.data?.files.map((path) => (
                  <Button
                    key={path}
                    type="button"
                    size="sm"
                    variant={selectedPath === path ? "secondary" : "ghost"}
                    aria-pressed={selectedPath === path}
                    onClick={() => setSelectedPath(path)}
                    className="shrink-0"
                  >
                    {path}
                  </Button>
                ))}
              </div>
              {preview?.isError ? (
                <div className="flex items-center gap-2">
                  <p role="alert" className="text-sm text-destructive">
                    Could not load {selectedPath}.
                  </p>
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => preview.refetch()}
                  >
                    Retry
                  </Button>
                </div>
              ) : preview?.isPending ? (
                <>
                  <output className="sr-only">Loading file</output>
                  <Skeleton className="h-48 w-full" />
                </>
              ) : preview?.data?.content === null ? (
                <p className="text-sm text-muted-foreground">
                  {preview.data.unavailableReason ?? "Preview unavailable."}
                </p>
              ) : (
                <pre className="max-h-72 overflow-auto whitespace-pre-wrap break-words rounded-lg bg-muted p-3 text-xs">
                  {preview?.data?.content}
                </pre>
              )}
            </>
          )}
        </section>
        <DialogFooter>
          <Button variant="ghost" onClick={onClose} disabled={resolving}>
            Cancel
          </Button>
          <Button
            variant="destructive"
            onClick={onConfirm}
            disabled={resolving || !keptSkillId}
          >
            {resolving
              ? "Resolving…"
              : `Keep this copy and move ${others} aside`}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
