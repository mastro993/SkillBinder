import { useState, type FormEvent } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  CloudIcon,
  GitBranchIcon,
  RefreshIcon,
  UnplugIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { PageHeader } from "@/components/layout/page-header";
import { Skeleton } from "@/components/ui/skeleton";
import type { GitSyncState } from "@/types";
import {
  gitSyncStatusQuery,
  useGitSync,
  useGitSyncConnect,
  useGitSyncDisconnect,
  useGitSyncPull,
  useGitSyncPush,
  useGitSyncRefresh,
} from "../hooks/queries";

const stateCopy = {
  notConfigured: {
    label: "Remote not connected",
    description:
      "Your library is local only. Connect a remote when you want to share it.",
    variant: "outline",
  },
  synced: {
    label: "Up to date",
    description: "Local library and remote contain the same managed content.",
    variant: "success",
  },
  needsPull: {
    label: "Pull available",
    description: "Remote has changes waiting for this library.",
    variant: "warning",
  },
  needsPush: {
    label: "Push available",
    description: "Local library has changes ready to share.",
    variant: "warning",
  },
  needsSync: {
    label: "Sync needed",
    description:
      "Both sides changed. Sync needs an explicit review before it can continue.",
    variant: "destructive",
  },
} satisfies Record<
  GitSyncState,
  {
    label: string;
    description: string;
    variant: "success" | "warning" | "destructive" | "outline";
  }
>;

export function GitSyncView() {
  const status = useQuery(gitSyncStatusQuery);
  const [remote, setRemote] = useState("");
  const [branch, setBranch] = useState("main");
  const [formError, setFormError] = useState<string | null>(null);
  const connect = useGitSyncConnect();
  const refresh = useGitSyncRefresh();
  const pull = useGitSyncPull();
  const push = useGitSyncPush();
  const sync = useGitSync();
  const disconnect = useGitSyncDisconnect();
  const current = status.data;
  const copy = current ? stateCopy[current.state] : null;
  const operation =
    refresh.isPending ||
    pull.isPending ||
    push.isPending ||
    sync.isPending ||
    disconnect.isPending;
  const error =
    formError ??
    status.error?.message ??
    connect.error?.message ??
    refresh.error?.message ??
    pull.error?.message ??
    push.error?.message ??
    sync.error?.message ??
    disconnect.error?.message;

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!remote.trim()) {
      setFormError("Enter a Git remote URL.");
      return;
    }
    if (!branch.trim()) {
      setFormError("Enter a branch name.");
      return;
    }
    setFormError(null);
    connect.mutate({ remote: remote.trim(), branch: branch.trim() });
  }

  return (
    <section className="px-13 py-11.5">
      <PageHeader
        eyebrow="Portable library"
        title="Keep your library in sync"
        lead="Connect one Git remote to share skills and portable metadata across machines."
        action={
          current && copy ? (
            <Badge variant={copy.variant}>{copy.label}</Badge>
          ) : null
        }
      />

      {status.isPending ? (
        <>
          <output className="sr-only">Reading sync status…</output>
          <Card className="max-w-3xl">
            <CardHeader>
              <Skeleton className="h-5 w-32" />
              <Skeleton className="h-4 w-72" />
            </CardHeader>
            <CardContent>
              <div className="grid gap-3">
                <Skeleton className="h-3 w-20" />
                <Skeleton className="h-4 w-64" />
                <Skeleton className="h-4 w-48" />
              </div>
            </CardContent>
            <CardFooter className="flex-wrap justify-between">
              <div className="flex flex-wrap gap-2">
                <Skeleton className="h-10 w-36" />
                <Skeleton className="h-10 w-28" />
              </div>
              <Skeleton className="h-10 w-32" />
            </CardFooter>
          </Card>
        </>
      ) : null}

      {current?.state === "notConfigured" ? (
        <Card>
          <CardHeader>
            <CardTitle>Connect a Git remote</CardTitle>
            <CardDescription>
              Uses your existing Git authentication. SkillBinder never stores
              credentials.
            </CardDescription>
          </CardHeader>
          <form onSubmit={submit}>
            <CardContent>
              <div className="grid gap-5">
                <div className="grid gap-2">
                  <label htmlFor="git-remote" className="text-sm font-medium">
                    Remote URL
                  </label>
                  <Input
                    id="git-remote"
                    value={remote}
                    onChange={(event) => setRemote(event.target.value)}
                    placeholder="https://github.com/you/skills.git"
                    autoComplete="url"
                    spellCheck={false}
                  />
                  <p className="text-xs text-muted-foreground">
                    HTTPS and SSH remotes supported.
                  </p>
                </div>
                <div className="grid gap-2">
                  <label htmlFor="git-branch" className="text-sm font-medium">
                    Branch
                  </label>
                  <Input
                    id="git-branch"
                    value={branch}
                    onChange={(event) => setBranch(event.target.value)}
                    placeholder="main"
                    spellCheck={false}
                  />
                </div>
              </div>
            </CardContent>
            <CardFooter className="justify-end">
              <Button type="submit" disabled={connect.isPending}>
                <HugeiconsIcon
                  icon={CloudIcon}
                  data-icon="inline-start"
                  aria-hidden="true"
                />
                {connect.isPending ? "Connecting…" : "Connect remote"}
              </Button>
            </CardFooter>
          </form>
        </Card>
      ) : current ? (
        <div className="grid max-w-3xl gap-4">
          <Card>
            <CardHeader>
              <div className="flex items-start justify-between gap-4">
                <div className="grid gap-1">
                  <CardTitle>{copy?.label}</CardTitle>
                  <CardDescription>{copy?.description}</CardDescription>
                </div>
                <HugeiconsIcon
                  icon={GitBranchIcon}
                  className="mt-1 text-muted-foreground"
                  aria-hidden="true"
                />
              </div>
            </CardHeader>
            <CardContent>
              <div className="grid gap-3 text-sm">
                <div className="grid gap-1">
                  <span className="text-xs font-medium tracking-wide text-muted-foreground uppercase">
                    Remote
                  </span>
                  <code className="break-all text-foreground">
                    {current.remote}
                  </code>
                </div>
                <div className="flex flex-wrap gap-x-6 gap-y-2 text-muted-foreground">
                  <span>
                    Branch{" "}
                    <strong className="font-medium text-foreground">
                      {current.branch}
                    </strong>
                  </span>
                  {current.ahead > 0 ? (
                    <span>
                      {current.ahead} local commit
                      {current.ahead === 1 ? "" : "s"} ahead
                    </span>
                  ) : null}
                  {current.behind > 0 ? (
                    <span>
                      {current.behind} remote commit
                      {current.behind === 1 ? "" : "s"} behind
                    </span>
                  ) : null}
                </div>
              </div>
            </CardContent>
            <CardFooter className="flex-wrap justify-between">
              <div className="flex flex-wrap gap-2">
                <Button
                  variant="outline"
                  onClick={() => refresh.mutate()}
                  disabled={operation}
                >
                  <HugeiconsIcon
                    icon={RefreshIcon}
                    data-icon="inline-start"
                    aria-hidden="true"
                  />
                  Refresh status
                </Button>
                {current.state === "needsPull" ? (
                  <Button onClick={() => pull.mutate()} disabled={operation}>
                    Pull changes
                  </Button>
                ) : null}
                {current.state === "needsPush" && !current.hasLocalChanges ? (
                  <Button onClick={() => push.mutate()} disabled={operation}>
                    Push changes
                  </Button>
                ) : null}
                {current.state === "needsSync" || current.hasLocalChanges ? (
                  <Button onClick={() => sync.mutate()} disabled={operation}>
                    Sync changes
                  </Button>
                ) : null}
              </div>
              <Button
                variant="ghost"
                onClick={() => disconnect.mutate()}
                disabled={operation}
              >
                <HugeiconsIcon
                  icon={UnplugIcon}
                  data-icon="inline-start"
                  aria-hidden="true"
                />
                Disconnect
              </Button>
            </CardFooter>
          </Card>
          <Alert variant="muted">
            <HugeiconsIcon icon={GitBranchIcon} aria-hidden="true" />
            <AlertTitle>Only library content travels</AlertTitle>
            <AlertDescription>
              Skill payloads in <code>skills/</code> and portable metadata in{" "}
              <code>.skillbinder.json</code> are managed. Credentials and device
              settings stay local.
            </AlertDescription>
          </Alert>
        </div>
      ) : null}

      {error ? (
        <Alert variant="destructive" className="mt-4 max-w-3xl">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      ) : null}
    </section>
  );
}
