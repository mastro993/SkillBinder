import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { BookOpen02Icon, PlusSignIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { bootstrapQuery } from "@/lib/bootstrap-query";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
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
import { PageHeader } from "@/components/layout/page-header";
import type { ValidationStatus } from "@/types";
import { libraryListQuery } from "../hooks/queries";

const validationVariants = {
  valid: "success",
  warning: "warning",
  invalid: "destructive",
  blocked: "destructive",
} satisfies Record<ValidationStatus, "success" | "warning" | "destructive">;

export function LibraryView() {
  const bootstrap = useQuery(bootstrapQuery);
  const library = useQuery(libraryListQuery);
  if (bootstrap.isPending || library.isPending)
    return (
      <p className="px-13 py-12 text-muted-foreground">Loading library…</p>
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
  return (
    <section className="px-13 py-11.5">
      <PageHeader
        eyebrow="Canonical collection"
        title="Library"
        action={
          <Button disabled>
            <HugeiconsIcon icon={PlusSignIcon} aria-hidden="true" /> New skill
          </Button>
        }
      />
      {data.hasUncommittedChanges ? (
        <Alert variant="warning" role="note" className="mb-6">
          <AlertDescription>
            Imported content is not committed yet.
          </AlertDescription>
        </Alert>
      ) : null}
      {data.skills.length === 0 ? (
        <Empty variant="outline" className="min-h-[390px]">
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <span className="text-primary">
                <HugeiconsIcon icon={BookOpen02Icon} aria-hidden="true" />
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
        <div className="grid grid-cols-[repeat(auto-fit,minmax(280px,1fr))] gap-4">
          {data.skills.map((skill) => (
            <Card key={skill.skillId}>
              <CardHeader>
                <CardTitle>{skill.displayName ?? skill.slug}</CardTitle>
                <CardDescription>{skill.slug}</CardDescription>
                <CardAction>
                  <Badge variant={validationVariants[skill.validation.status]}>
                    {skill.validation.status}
                  </Badge>
                </CardAction>
              </CardHeader>
              <CardContent>
                <p className="min-h-[42px] text-sm text-muted-foreground">
                  {skill.description ?? "No description"}
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
                      <span className="break-words">{source.displayPath}</span>
                      <span className="text-muted-foreground">
                        {source.readerAgentIds.join(", ") || "Unknown reader"}
                      </span>
                    </li>
                  ))}
                </ul>
              </CardFooter>
            </Card>
          ))}
        </div>
      )}
    </section>
  );
}
