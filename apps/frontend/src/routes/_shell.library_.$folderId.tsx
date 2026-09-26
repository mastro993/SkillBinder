import { createFileRoute } from "@tanstack/react-router";
import { LibraryView } from "@/features/library/screens/library-view";

export const Route = createFileRoute("/_shell/library_/$folderId")({
  component: () => <LibraryView folderId={Route.useParams().folderId} />,
});
