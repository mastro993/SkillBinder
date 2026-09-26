import { createFileRoute } from "@tanstack/react-router";
import { GitSyncView } from "@/features/git-sync/screens/git-sync-view";

export const Route = createFileRoute("/_shell/git")({
  component: GitSyncView,
});
