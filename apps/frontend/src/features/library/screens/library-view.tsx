import { useQuery } from "@tanstack/react-query";
import { useRef, useState } from "react";
import { Link } from "@tanstack/react-router";
import { bootstrapQuery } from "@/lib/bootstrap-query";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Empty,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { Skeleton } from "@/components/ui/skeleton";
import { PageHeader } from "@/components/layout/page-header";
import type { ValidationStatus } from "@/types";
import { SlugConflictDialog } from "../components/slug-conflict-dialog";
import { payloadPath, slugConflictGroups } from "../lib/conflicts";
import { libraryListQuery, useResolveSlugConflict } from "../hooks/queries";
import {
  OrganizationControls,
  includesFolder,
  type FolderScope,
  type OrganizationControlsHandle,
} from "./organization-controls";

const validationVariants = {
  valid: "success",
  warning: "warning",
  invalid: "destructive",
  blocked: "destructive",
} satisfies Record<ValidationStatus, "success" | "warning" | "destructive">;

export function LibraryView() {
  const controlsRef = useRef<OrganizationControlsHandle>(null);
  const [scope, setScope] = useState<FolderScope>({ kind: "all" });
  const [search, setSearch] = useState("");
  const [activeTags, setActiveTags] = useState<string[]>([]);
  const [matchAny, setMatchAny] = useState(false);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const bootstrap = useQuery(bootstrapQuery);
  const library = useQuery(libraryListQuery);
  const resolve = useResolveSlugConflict();
  const [openSlug, setOpenSlug] = useState<string | null>(null);
  const [keptSkillId, setKeptSkillId] = useState<string | null>(null);
  if (bootstrap.isPending || library.isPending)
    return (
      <section className="px-13 py-11.5">
        <PageHeader
          eyebrow="Canonical collection"
          title="Library"
          action={<Button disabled>New skill</Button>}
        />
        <output className="sr-only">Loading library…</output>
        <div className="grid grid-cols-[repeat(auto-fit,minmax(280px,1fr))] gap-4">
          {[0, 1, 2, 3, 4, 5].map((slot) => (
            <Card key={slot}>
              <CardHeader>
                <Skeleton className="h-5 w-40" />
                <Skeleton className="h-4 w-24" />
                <CardAction>
                  <Skeleton variant="pill" className="h-5 w-16" />
                </CardAction>
              </CardHeader>
              <CardContent>
                <div className="grid gap-2">
                  <Skeleton className="h-4 w-full" />
                  <Skeleton className="h-4 w-3/4" />
                  <Skeleton className="h-3 w-32" />
                </div>
              </CardContent>
              <CardFooter className="block">
                <Skeleton className="h-4 w-full" />
              </CardFooter>
            </Card>
          ))}
        </div>
      </section>
    );
  if (bootstrap.isError || library.isError)
    return (
      <p className="px-13 py-12 text-destructive">Library state unavailable.</p>
    );
  if (!bootstrap.data.onboarding.completed) {
    return (
      <div className="px-13 py-11.5">
        <Empty variant="outline" className="min-h-[390px]">
          <EmptyHeader>
            <EmptyTitle>Setup is not finished</EmptyTitle>
          </EmptyHeader>
          <Button render={<Link to="/onboarding" />}>Resume setup</Button>
        </Empty>
      </div>
    );
  }
  const data = library.data;
  const conflicts = slugConflictGroups(data.skills);
  const sharedSlugs = new Set(conflicts.map((group) => group.slug));
  const open = conflicts.find((group) => group.slug === openSlug) ?? null;
  const visibleSkills = data.skills.filter((skill) => {
    if (scope.kind === "unfiled" && skill.folderId !== null) return false;
    if (
      scope.kind === "folder" &&
      !includesFolder(skill.folderId, scope.id, data.folders)
    )
      return false;
    if (
      activeTags.length &&
      !(matchAny
        ? activeTags.some((id) => skill.tagIds.includes(id))
        : activeTags.every((id) => skill.tagIds.includes(id)))
    )
      return false;
    const query = search.trim().toLocaleLowerCase();
    return (
      !query ||
      [skill.displayName, skill.slug, skill.description].some((value) =>
        value?.toLocaleLowerCase().includes(query),
      )
    );
  });
  return (
    <section className="px-13 py-11.5">
      <PageHeader
        eyebrow="Canonical collection"
        title="Library"
        action={<Button disabled>New skill</Button>}
      />
      {data.hasUncommittedChanges ? (
        <Alert variant="warning" role="note" className="mb-6">
          <AlertDescription>
            Library changes are not committed yet.
          </AlertDescription>
        </Alert>
      ) : null}
      {conflicts.length ? (
        <Alert variant="warning" role="note" className="mb-6">
          <AlertDescription>
            {conflicts.length === 1
              ? "One slug names two different skills."
              : `${conflicts.length} slugs name two different skills.`}{" "}
            Choose which copy to keep in each.
            {data.pendingResolution
              ? " A previous resolution was interrupted; the next attempt finishes it."
              : ""}
          </AlertDescription>
          <ul className="mt-3 grid gap-2">
            {conflicts.map((group) => (
              <li
                className="flex items-center justify-between gap-3 text-sm"
                key={group.slug}
              >
                <span>
                  <code>{group.slug}</code> · {group.skills.length} copies
                </span>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => {
                    setOpenSlug(group.slug);
                    setKeptSkillId(group.skills[0]?.skillId ?? null);
                  }}
                >
                  Choose which to keep
                </Button>
              </li>
            ))}
          </ul>
        </Alert>
      ) : null}
      <div className="grid grid-cols-[220px_minmax(0,1fr)] gap-6">
        <OrganizationControls
          ref={controlsRef}
          data={data}
          scope={scope}
          setScope={setScope}
          selectedIds={selectedIds}
          clearSelection={() => setSelectedIds([])}
          activeTags={activeTags}
          setActiveTags={setActiveTags}
          matchAny={matchAny}
          setMatchAny={setMatchAny}
        />
        <div className="grid content-start gap-4">
          {data.skills.length === 0 ? (
            <Empty variant="outline" className="min-h-[390px]">
              <EmptyHeader>
                <EmptyMedia variant="icon">
                  <span className="text-primary" aria-hidden="true">
                    Skills
                  </span>
                </EmptyMedia>
                <EmptyTitle>Your library is ready</EmptyTitle>
              </EmptyHeader>
              <p className="max-w-[490px] text-muted-foreground">
                No skills imported yet. Visit Discovery to inspect local skill
                folders.
              </p>
            </Empty>
          ) : (
            <>
              <div className="flex items-center gap-3">
                <Input
                  aria-label="Search skills"
                  placeholder="Search skills"
                  value={search}
                  onChange={(event) => setSearch(event.target.value)}
                />
                {search || activeTags.length || scope.kind !== "all" ? (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => {
                      setSearch("");
                      setActiveTags([]);
                      setScope({ kind: "all" });
                    }}
                  >
                    Clear filters
                  </Button>
                ) : null}
              </div>
              {visibleSkills.length === 0 ? (
                <div className="rounded-lg border p-8 text-center">
                  <p className="font-medium">No matching skills</p>
                  <p className="mt-1 text-sm text-muted-foreground">
                    Try another folder, tag, or search.
                  </p>
                  <Button
                    variant="outline"
                    size="sm"
                    className="mt-4"
                    onClick={() => {
                      setSearch("");
                      setActiveTags([]);
                      setScope({ kind: "all" });
                    }}
                  >
                    Clear filters
                  </Button>
                </div>
              ) : null}
              <div className="grid grid-cols-[repeat(auto-fit,minmax(280px,1fr))] items-start gap-4">
                {visibleSkills.map((skill) => (
                  <Card key={skill.skillId}>
                    <CardHeader>
                      <div className="flex items-center gap-2">
                        <Checkbox
                          aria-label={`Select ${skill.displayName ?? skill.slug}`}
                          checked={selectedIds.includes(skill.skillId)}
                          onCheckedChange={(checked) =>
                            setSelectedIds(
                              checked
                                ? [...selectedIds, skill.skillId]
                                : selectedIds.filter(
                                    (id) => id !== skill.skillId,
                                  ),
                            )
                          }
                        />
                        <CardTitle>{skill.displayName ?? skill.slug}</CardTitle>
                      </div>
                      <CardDescription>
                        {sharedSlugs.has(skill.slug)
                          ? payloadPath(skill)
                          : skill.slug}
                      </CardDescription>
                      <CardAction>
                        <Badge
                          variant={validationVariants[skill.validation.status]}
                        >
                          {skill.validation.status}
                        </Badge>
                      </CardAction>
                    </CardHeader>
                    <CardContent>
                      <p className="min-h-[42px] text-sm text-muted-foreground">
                        {skill.description ?? "No description"}
                      </p>
                      <Button
                        variant="outline"
                        size="xs"
                        onClick={() =>
                          controlsRef.current?.organizeSkill(skill.skillId)
                        }
                      >
                        Organize
                      </Button>
                      <p className="mt-2 text-xs text-muted-foreground">
                        {data.folders.find(
                          (folder) => folder.id === skill.folderId,
                        )?.name ?? "Unfiled"}
                        {skill.tagIds.length
                          ? ` · ${skill.tagIds.map((id) => data.tags.find((tag) => tag.id === id)?.name ?? id).join(", ")}`
                          : ""}
                      </p>
                      <p className="mt-2 text-xs text-muted-foreground">
                        {skill.fileCount} files · {skill.totalBytes} bytes
                      </p>
                      {skill.validation.messages.map((message) => (
                        <p
                          className="mt-1 text-xs text-muted-foreground"
                          key={message.code}
                        >
                          {message.message}
                        </p>
                      ))}
                    </CardContent>
                    <CardFooter className="block">
                      <ul className="grid gap-2">
                        {skill.sources.map((source) => (
                          <li
                            className="grid gap-0.5 border-b pb-2 text-xs last:border-b-0 last:pb-0"
                            key={source.displayPath}
                          >
                            <span className="break-words">
                              {source.displayPath}
                            </span>
                            <span className="text-muted-foreground">
                              {source.readerAgentIds.join(", ") ||
                                "Unknown reader"}
                            </span>
                          </li>
                        ))}
                      </ul>
                    </CardFooter>
                  </Card>
                ))}
              </div>
            </>
          )}
        </div>
      </div>
      {open && keptSkillId ? (
        <SlugConflictDialog
          slug={open.slug}
          skills={open.skills}
          keptSkillId={keptSkillId}
          resolving={resolve.isPending}
          onKeepChange={setKeptSkillId}
          onClose={() => {
            setOpenSlug(null);
            setKeptSkillId(null);
          }}
          onConfirm={() =>
            resolve.mutate(
              {
                slug: open.slug,
                keepSkillId: keptSkillId,
                expectedSkillIds: open.skills.map((skill) => skill.skillId),
              },
              {
                onSuccess: () => {
                  setOpenSlug(null);
                  setKeptSkillId(null);
                },
              },
            )
          }
        />
      ) : null}
    </section>
  );
}
