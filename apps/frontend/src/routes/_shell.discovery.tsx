import { createFileRoute } from "@tanstack/react-router";
import { DiscoveryView } from "@/features/discovery/screens/discovery-view";

export const Route = createFileRoute("/_shell/discovery")({
  component: DiscoveryView,
});
