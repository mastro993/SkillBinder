import { createFileRoute, redirect } from "@tanstack/react-router";
import { bootstrapQuery } from "@/lib/bootstrap-query";

export const Route = createFileRoute("/")({
  beforeLoad: async ({ context }) => {
    const bootstrap = await context.queryClient.ensureQueryData(bootstrapQuery);
    throw redirect({
      to: bootstrap.onboarding.completed ? "/discovery" : "/onboarding",
    });
  },
});
