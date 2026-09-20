import { createFileRoute } from "@tanstack/react-router";
import { LibraryView } from "@/features/library";

export const Route = createFileRoute("/_shell/library")({
  component: LibraryView,
});
