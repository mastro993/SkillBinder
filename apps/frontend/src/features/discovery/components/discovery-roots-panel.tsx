import { useState } from "react";
import { Plus } from "lucide-react";
import type { RootView } from "@/types";
import { Button } from "@/components/ui/button";

export function DiscoveryRootsPanel({
  roots,
  adding,
  error,
  onAdd,
  onUpdate,
  onRemove,
}: {
  roots: RootView[];
  adding: boolean;
  error: string | null;
  onAdd: () => void;
  onUpdate: (rootId: string, label: string, enabled: boolean) => void;
  onRemove: (rootId: string) => void;
}) {
  return (
    <section className="candidate-panel" aria-labelledby="roots-title">
      <div className="section-heading">
        <div>
          <h2 id="roots-title">Project-search roots</h2>
          <p>
            Folders SkillBinder walks in addition to the known agent locations.
          </p>
        </div>
        <span>{roots.length} registered</span>
      </div>
      {error ? (
        <p className="inline-error" role="alert">
          {error}
        </p>
      ) : null}
      {roots.length ? (
        <ul className="source-list">
          {roots.map((root) => (
            <RootRow
              key={root.rootId}
              root={root}
              onUpdate={onUpdate}
              onRemove={onRemove}
            />
          ))}
        </ul>
      ) : (
        <p className="discovery-disclosure">
          No project-search root yet. SkillBinder already inspects the skill
          folders of the agents it knows. Add a folder here to scan it as a
          project: nested projects, worktree markers, and monorepo packages are
          walked within the depth and entry limits, and every skipped folder is
          reported with its reason. Import copies content and leaves originals
          unchanged.
        </p>
      )}
      <div className="selection-actions">
        <Button onClick={onAdd} disabled={adding}>
          <Plus size={16} /> {adding ? "Choosing a folder…" : "Add folder"}
        </Button>
      </div>
    </section>
  );
}

function RootRow({
  root,
  onUpdate,
  onRemove,
}: {
  root: RootView;
  onUpdate: (rootId: string, label: string, enabled: boolean) => void;
  onRemove: (rootId: string) => void;
}) {
  const [label, setLabel] = useState(root.label);
  const rename = label.trim();
  return (
    <li className="setting-row">
      <div>
        <strong>{root.label}</strong>
        <span className="table-path">{root.displayPath}</span>
        <span className="table-secondary">
          {root.enabled ? "Scanned" : "Disabled"}
        </span>
      </div>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          if (rename.length === 0 || rename === root.label) return;
          onUpdate(root.rootId, rename, root.enabled);
        }}
      >
        <input
          aria-label={`Label for ${root.displayPath}`}
          value={label}
          onChange={(event) => setLabel(event.target.value)}
        />
        <Button
          variant="ghost"
          type="submit"
          disabled={rename.length === 0 || rename === root.label}
        >
          Rename
        </Button>
      </form>
      <label className="confirm-toggle">
        <input
          type="checkbox"
          checked={root.enabled}
          aria-label={`Scan ${root.label}`}
          onChange={(event) =>
            onUpdate(root.rootId, root.label, event.target.checked)
          }
        />{" "}
        Enabled
      </label>
      <Button
        variant="ghost"
        aria-label={`Remove ${root.label}`}
        onClick={() => onRemove(root.rootId)}
      >
        Remove
      </Button>
    </li>
  );
}
