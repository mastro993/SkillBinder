import { createFileRoute } from "@tanstack/react-router";
import { SettingsView } from "@/features/settings/screens/settings-view";

export const Route = createFileRoute("/_shell/settings")({
  component: SettingsView,
});
