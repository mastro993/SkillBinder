import { createFileRoute } from "@tanstack/react-router";
import { DiscoveryView } from "@/features/discovery";

export const Route = createFileRoute("/_shell/discovery")({
  component: DiscoveryView,
});
