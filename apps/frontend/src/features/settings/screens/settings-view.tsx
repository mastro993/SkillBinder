import { useQuery } from "@tanstack/react-query";
import { bootstrapQuery } from "@/lib/bootstrap-query";
import { DiscoveryRootsPanel } from "@/features/discovery/components/discovery-roots-panel";
import {
  rootsQuery,
  useAddRoot,
  useRemoveRoot,
  useUpdateRoot,
} from "@/features/discovery/hooks/queries";

export function SettingsView() {
  const bootstrap = useQuery(bootstrapQuery);
  const roots = useQuery(rootsQuery);
  const addRoot = useAddRoot();
  const updateRoot = useUpdateRoot();
  const removeRoot = useRemoveRoot();

  const rootError =
    addRoot.error?.message ??
    updateRoot.error?.message ??
    removeRoot.error?.message ??
    null;

  return (
    <section className="page">
      <header className="page-header">
        <div>
          <p className="eyebrow">This device</p>
          <h1>Settings</h1>
        </div>
      </header>
      <div className="settings-list">
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
      </div>

      {roots.isError ? (
        <p className="inline-error" role="alert">
          The configured project-search roots could not be read.{" "}
          {roots.error.message}
        </p>
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
    <div className="setting-row">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}
