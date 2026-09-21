import { useQuery } from "@tanstack/react-query";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemTitle,
} from "@/components/ui/item";
import { PageHeader } from "@/components/layout/page-header";
import { bootstrapQuery } from "@/lib/bootstrap-query";
import { DiscoveryRootsPanel } from "@/features/discovery/components/discovery-roots-panel";
import {
  rootsQuery,
  useAddRoot,
  useRemoveRoot,
  useUpdateRoot,
} from "@/features/discovery/hooks/queries";
import { useRevealLogs } from "@/features/settings/hooks/reveal-logs";

export function SettingsView() {
  const bootstrap = useQuery(bootstrapQuery);
  const roots = useQuery(rootsQuery);
  const addRoot = useAddRoot();
  const updateRoot = useUpdateRoot();
  const removeRoot = useRemoveRoot();
  const revealLogs = useRevealLogs();

  const rootError =
    addRoot.error?.message ??
    updateRoot.error?.message ??
    removeRoot.error?.message ??
    null;

  return (
    <section className="px-13 py-11.5">
      <PageHeader eyebrow="This device" title="Settings" />
      <ul className="grid gap-2">
        <Setting
          label="Application"
          value={
            bootstrap.data
              ? `SkillBinder ${bootstrap.data.appVersion}`
              : "Loading…"
          }
        />
        <Setting
          label="Git"
          value={
            bootstrap.data?.git.version
              ? `Ready · ${bootstrap.data.git.version}`
              : "Needs attention"
          }
        />
        <Setting
          label="Library"
          value={
            bootstrap.data?.libraryState === "ready"
              ? "Local library ready"
              : "Not created"
          }
        />
        <Setting label="Remote sync" value="Not connected · optional" />
        <Item variant="outline" render={<li />}>
          <ItemContent>
            <ItemTitle>Logs</ItemTitle>
            <ItemDescription>
              {revealLogs.data?.path ??
                (revealLogs.isPending ? "Opening…" : "On this device")}
            </ItemDescription>
          </ItemContent>
          <ItemActions>
            <Button
              variant="outline"
              onClick={() => revealLogs.mutate()}
              disabled={revealLogs.isPending}
            >
              Open log folder
            </Button>
          </ItemActions>
        </Item>
      </ul>

      {revealLogs.error ? (
        <Alert variant="destructive" className="mt-4">
          <AlertDescription>{revealLogs.error.message}</AlertDescription>
        </Alert>
      ) : null}

      {roots.isError ? (
        <Alert variant="destructive" className="mt-4">
          <AlertDescription>
            The configured project-search roots could not be read.{" "}
            {roots.error.message}
          </AlertDescription>
        </Alert>
      ) : roots.data ? (
        <DiscoveryRootsPanel
          roots={roots.data.roots}
          adding={addRoot.isPending}
          error={rootError}
          onAdd={() => addRoot.mutate()}
          onUpdate={(rootId, label, enabled) =>
            updateRoot.mutate({ rootId, label, enabled })
          }
          onRemove={(rootId) => removeRoot.mutate(rootId)}
        />
      ) : null}
    </section>
  );
}

function Setting({ label, value }: { label: string; value: string }) {
  return (
    <Item variant="outline" render={<li />}>
      <ItemContent>
        <ItemTitle>{label}</ItemTitle>
        <ItemDescription>{value}</ItemDescription>
      </ItemContent>
    </Item>
  );
}
