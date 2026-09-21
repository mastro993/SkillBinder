import { useState } from "react";
import { Plus } from "lucide-react";
import type { RootView } from "@/types";
import { Alert, AlertDescription } from "@/components/ui/alert";
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
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Item, ItemContent, ItemTitle } from "@/components/ui/item";

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
    <Card aria-labelledby="roots-title" className="mt-6">
      <CardHeader>
        <CardTitle id="roots-title">Project-search roots</CardTitle>
        <CardDescription>
          Folders SkillBinder searches in addition to the known agent locations.
          Inside them only the skill folders its agents use are read.
        </CardDescription>
        <CardAction>
          <span className="text-xs text-muted-foreground">
            {roots.length} registered
          </span>
        </CardAction>
      </CardHeader>
      <CardContent>
        {error ? (
          <Alert variant="destructive" className="mb-4">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        ) : null}
        {roots.length ? (
          <ul className="grid gap-2">
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
          <p className="text-sm text-muted-foreground">
            No project-search root yet. SkillBinder already inspects the skill
            folders of the agents it knows. Add a folder here and the project
            skill folders inside it are read, within the depth and entry limits,
            and every skipped folder is reported with its reason. Import copies
            content and leaves originals unchanged.
          </p>
        )}
      </CardContent>
      <CardFooter className="justify-end">
        <Button onClick={onAdd} disabled={adding}>
          <Plus aria-hidden="true" />{" "}
          {adding ? "Choosing a folder…" : "Add folder"}
        </Button>
      </CardFooter>
    </Card>
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
    <Item variant="outline" render={<li />}>
      <ItemContent>
        <ItemTitle>{root.label}</ItemTitle>
        <p className="max-w-[360px] break-words text-xs text-muted-foreground">
          {root.displayPath}
        </p>
        <p className="text-xs text-muted-foreground">
          {root.enabled ? "Scanned" : "Disabled"}
        </p>
      </ItemContent>
      <form
        className="flex items-center gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          if (rename.length === 0 || rename === root.label) return;
          onUpdate(root.rootId, rename, root.enabled);
        }}
      >
        <Input
          aria-label={`Label for ${root.displayPath}`}
          className="w-48"
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
      <div className="flex items-center gap-2">
        <Checkbox
          checked={root.enabled}
          aria-label={`Scan ${root.label}`}
          onCheckedChange={(checked) =>
            onUpdate(root.rootId, root.label, checked === true)
          }
        />
        <span className="text-sm">Enabled</span>
      </div>
      <Button
        variant="ghost"
        aria-label={`Remove ${root.label}`}
        onClick={() => onRemove(root.rootId)}
      >
        Remove
      </Button>
    </Item>
  );
}
