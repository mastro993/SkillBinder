import { createFileRoute } from "@tanstack/react-router";
import { OnboardingView } from "@/features/onboarding";

export const Route = createFileRoute("/onboarding")({
  component: OnboardingView,
});
