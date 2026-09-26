import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { getDesktopClient } from "@/commands/client";
import { PageHeader } from "@/components/layout/page-header";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { bootstrapQuery } from "@/lib/bootstrap-query";
import { libraryListQuery } from "@/features/library/hooks/queries";

const statusLabels = new Map([
  ["installed", "Installed"],
  ["missing", "Missing copy"],
  ["changed", "Changed copy"],
  ["unavailable", "Unavailable"],
  ["unmanaged", "Unmanaged target"],
  ["sourceMissing", "Source missing"],
  ["sourceChanged", "Source changed"],
]);

function toggle(items: string[], item: string, checked: boolean) {
  return checked ? [...items, item] : items.filter((value) => value !== item);
}

export function BindingsView() {
  const queryClient = useQueryClient();
  const bootstrap = useQuery(bootstrapQuery);
  const library = useQuery(libraryListQuery);
  const roots = useQuery({
    queryKey: ["bindingRoots"],
    queryFn: async () => (await getDesktopClient()).rootsList(),
  });
  const bindings = useQuery({
    queryKey: ["bindings"],
    queryFn: async () => (await getDesktopClient()).bindingsList(),
    refetchInterval: 5000,
    refetchOnWindowFocus: true,
  });
  const [scope, setScope] = useState<"global" | "project">("global");
  const [projectRootId, setProjectRootId] = useState<string | null>(null);
  const [skillIds, setSkillIds] = useState<string[]>([]);
  const [agentSelection, setAgentSelection] = useState<{
    key: string;
    ids: string[];
  } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const options = useQuery({
    queryKey: ["bindingOptions", scope, projectRootId],
    queryFn: async () =>
      (await getDesktopClient()).bindingsOptions(
        scope === "project" ? projectRootId : null,
      ),
    enabled: scope === "global" || projectRootId !== null,
  });
  const selectionKey = `${scope}:${projectRootId ?? ""}`;
  const agentIds =
    agentSelection?.key === selectionKey
      ? agentSelection.ids
      : (options.data?.agents
          .filter((agent) => agent.detected)
          .map((agent) => agent.agentId) ?? []);
  const pick = useMutation({
    mutationFn: async () => {
      const client = await getDesktopClient();
      const { grant } = await client.rootsPick();
      return grant
        ? (await client.rootsRegister(grant.grantId, null)).root
        : null;
    },
    onSuccess: (root) => {
      if (root) {
        setScope("project");
        setProjectRootId(root.rootId);
        void queryClient.invalidateQueries({ queryKey: ["bindingRoots"] });
      }
    },
    onError: (cause) => setError(String(cause)),
  });
  const create = useMutation({
    mutationFn: async () =>
      (await getDesktopClient()).bindingsCreate(
        skillIds,
        scope,
        scope === "project" ? projectRootId : null,
        agentIds,
      ),
    onSuccess: () => {
      setError(null);
      setSkillIds([]);
      void queryClient.invalidateQueries({ queryKey: ["bindings"] });
    },
    onError: (cause) => setError(String(cause)),
  });
  const repair = useMutation({
    mutationFn: async (bindingId: string) =>
      (await getDesktopClient()).bindingsRepair(bindingId),
    onSuccess: () => {
      setError(null);
      void queryClient.invalidateQueries({ queryKey: ["bindings"] });
    },
    onError: (cause) => setError(String(cause)),
  });

  if (
    bootstrap.isPending ||
    library.isPending ||
    roots.isPending ||
    bindings.isPending
  )
    return (
      <p className="px-13 py-12 text-muted-foreground">Loading bindings…</p>
    );
  if (bootstrap.isError || library.isError || roots.isError || bindings.isError)
    return (
      <p className="px-13 py-12 text-destructive">Bindings are unavailable.</p>
    );
  if (!bootstrap.data.onboarding.completed)
    return (
      <section className="px-13 py-11.5">
        <PageHeader eyebrow="Local copies" title="Bindings" />
        <p>Finish setup before creating bindings.</p>
        <Button render={<Link to="/onboarding" />}>Resume setup</Button>
      </section>
    );
  const skillNames = new Map(
    library.data.skills.map((skill) => [
      skill.skillId,
      skill.displayName ?? skill.slug,
    ]),
  );
  return (
    <section className="px-13 py-11.5">
      <PageHeader
        eyebrow="Local copies"
        title="Bindings"
        lead="Copy selected library skills into the folders your agents read. Each binding is an independent action."
      />
      {error ? (
        <Alert variant="destructive" className="mb-6">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      ) : null}
      <Card className="mb-8">
        <CardHeader>
          <CardTitle>Create binding</CardTitle>
        </CardHeader>
        <CardContent className="grid">
          <fieldset className="grid gap-3">
            <legend className="mb-2 font-semibold">Skills</legend>
            {library.data.skills.length === 0 ? (
              <p className="text-muted-foreground">
                Import skills in Discovery first.
              </p>
            ) : (
              library.data.skills.map((skill) => (
                <div key={skill.skillId} className="flex items-center gap-3">
                  <Checkbox
                    aria-label={skill.displayName ?? skill.slug}
                    disabled={
                      skill.validation.status === "invalid" ||
                      skill.validation.status === "blocked"
                    }
                    checked={skillIds.includes(skill.skillId)}
                    onCheckedChange={(checked) =>
                      setSkillIds((current) =>
                        toggle(current, skill.skillId, checked),
                      )
                    }
                  />
                  {skill.displayName ?? skill.slug}
                </div>
              ))
            )}
          </fieldset>
          <fieldset>
            <legend className="mb-2 font-semibold">Destination</legend>
            <div className="flex gap-2">
              <Button
                variant={scope === "global" ? "default" : "outline"}
                onClick={() => {
                  setScope("global");
                  setProjectRootId(null);
                }}
              >
                Global skills folders
              </Button>
              <Button
                variant={scope === "project" ? "default" : "outline"}
                onClick={() => setScope("project")}
              >
                Project folder
              </Button>
            </div>
            {scope === "project" ? (
              <div className="mt-3 grid gap-2">
                <div className="flex flex-wrap gap-2">
                  {roots.data.roots.map((root) => (
                    <Button
                      key={root.rootId}
                      variant={
                        projectRootId === root.rootId ? "default" : "outline"
                      }
                      onClick={() => setProjectRootId(root.rootId)}
                    >
                      {root.label}
                    </Button>
                  ))}
                </div>
                <Button
                  variant="outline"
                  onClick={() => pick.mutate()}
                  disabled={pick.isPending}
                >
                  Choose another project folder…
                </Button>
              </div>
            ) : null}
          </fieldset>
          <fieldset className="grid gap-3">
            <legend className="mb-2 font-semibold">Agents</legend>
            {scope === "project" && !projectRootId ? (
              <p className="text-muted-foreground">
                Choose a project folder to discover agents.
              </p>
            ) : (
              options.data?.agents
                .filter((agent) => agent.detected)
                .map((agent) => (
                  <div key={agent.agentId} className="flex items-start gap-3">
                    <Checkbox
                      aria-label={agent.displayName}
                      checked={agentIds.includes(agent.agentId)}
                      disabled={!agent.available}
                      onCheckedChange={(checked) =>
                        setAgentSelection({
                          key: selectionKey,
                          ids: toggle(agentIds, agent.agentId, checked),
                        })
                      }
                    />
                    <span>
                      <span className="block">
                        {agent.displayName}{" "}
                        {agent.detected ? (
                          <Badge variant="secondary">Detected</Badge>
                        ) : null}
                      </span>
                      <span className="break-all text-xs text-muted-foreground">
                        {agent.readerPath || "No skills folder configured"}
                      </span>
                    </span>
                  </div>
                ))
            )}
            {options.data?.agents.some(
              (agent) => !agent.detected && agent.available,
            ) ? (
              <details>
                <summary className="cursor-pointer text-sm font-medium">
                  Add other agents
                </summary>
                <div className="mt-3 grid gap-3">
                  {options.data.agents
                    .filter((agent) => !agent.detected && agent.available)
                    .map((agent) => (
                      <div
                        key={agent.agentId}
                        className="flex items-start gap-3"
                      >
                        <Checkbox
                          aria-label={agent.displayName}
                          checked={agentIds.includes(agent.agentId)}
                          onCheckedChange={(checked) =>
                            setAgentSelection({
                              key: selectionKey,
                              ids: toggle(agentIds, agent.agentId, checked),
                            })
                          }
                        />
                        <span>
                          <span className="block">{agent.displayName}</span>
                          <span className="break-all text-xs text-muted-foreground">
                            {agent.readerPath}
                          </span>
                        </span>
                      </div>
                    ))}
                </div>
              </details>
            ) : null}
            <p className="text-xs text-muted-foreground">
              Shared folders may also be read by agents you did not select. Each
              copy lists all known readers below.
            </p>
          </fieldset>
          <div>
            <Button
              disabled={
                create.isPending ||
                skillIds.length === 0 ||
                agentIds.length === 0 ||
                (scope === "project" && !projectRootId)
              }
              onClick={() => create.mutate()}
            >
              Create binding
            </Button>
          </div>
        </CardContent>
      </Card>
      <h2 className="mb-4 text-lg font-semibold">Created bindings</h2>
      {bindings.data.bindings.length === 0 ? (
        <p className="text-muted-foreground">No bindings yet.</p>
      ) : (
        <div className="grid gap-4">
          {bindings.data.bindings.map((binding) => (
            <Card key={binding.bindingId}>
              <CardHeader>
                <CardTitle>
                  {binding.skillIds
                    .map((id) => skillNames.get(id) ?? id)
                    .join(", ")}
                </CardTitle>
              </CardHeader>
              <CardContent className="grid">
                <p className="text-sm text-muted-foreground">
                  {binding.scope === "global"
                    ? "Global"
                    : (roots.data.roots.find(
                        (root) => root.rootId === binding.projectRootId,
                      )?.label ?? "Project")}{" "}
                  · {binding.agentIds.join(", ")}
                </p>
                {binding.targets.map((target) => (
                  <div
                    key={`${target.skillId}:${target.path}`}
                    className="flex items-start justify-between gap-4 border-t pt-3"
                  >
                    <div>
                      <p className="text-sm font-medium">
                        {skillNames.get(target.skillId) ?? target.skillId}
                      </p>
                      <p className="break-all text-xs text-muted-foreground">
                        {target.path}
                      </p>
                      <p className="text-xs text-muted-foreground">
                        Read by {target.readerAgentIds.join(", ")}
                      </p>
                    </div>
                    <Badge
                      variant={
                        target.status === "installed" ? "success" : "warning"
                      }
                    >
                      {statusLabels.get(target.status) ?? target.status}
                    </Badge>
                  </div>
                ))}
                {binding.targets.some(
                  (target) => target.status === "missing",
                ) ? (
                  <div>
                    <Button
                      variant="outline"
                      disabled={repair.isPending}
                      onClick={() => repair.mutate(binding.bindingId)}
                    >
                      Repair binding
                    </Button>
                  </div>
                ) : null}
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </section>
  );
}
