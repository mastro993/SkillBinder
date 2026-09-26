import { createFileRoute } from "@tanstack/react-router";
import { BindingsView } from "@/features/bindings/screens/bindings-view";

export const Route = createFileRoute("/_shell/bindings")({
  component: BindingsView,
});
