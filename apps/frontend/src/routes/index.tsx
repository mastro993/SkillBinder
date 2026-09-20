import { createFileRoute, redirect } from "@tanstack/react-router";
import { bootstrapQuery } from "@/app/bootstrap-query";

export const Route = createFileRoute("/")({
  beforeLoad: async ({ context }) => {
    const bootstrap = await context.queryClient.ensureQueryData(bootstrapQuery);
    throw redirect({
      to: bootstrap.onboarding.completed ? "/library" : "/onboarding",
    });
  },
});
