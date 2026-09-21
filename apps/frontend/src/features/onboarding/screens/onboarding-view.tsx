import { useMutation, useQuery } from "@tanstack/react-query";
import { Navigate, useNavigate } from "@tanstack/react-router";
import type { OnboardingStep } from "@/types";
import { bootstrapQuery } from "@/lib/bootstrap-query";
import { queryClient } from "@/lib/query-client";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { getDesktopClient } from "@/commands/client";
import {
  OnboardingStepContent,
  onboardingSteps,
  stepAfterRevalidation,
  stepSubtitle,
  stepTitle,
} from "../components/onboarding-steps";

export function OnboardingView() {
  const navigate = useNavigate();
  const bootstrap = useQuery(bootstrapQuery);
  const update = useMutation({
    mutationFn: async (step: OnboardingStep) =>
      (await getDesktopClient()).updateOnboardingProgress(step),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: bootstrapQuery.queryKey }),
  });
  const complete = useMutation({
    mutationFn: async () =>
      (await getDesktopClient()).completeLocalOnboarding(),
    onSuccess: async () => {
      await queryClient.invalidateQueries({
        queryKey: bootstrapQuery.queryKey,
      });
      await navigate({ to: "/discovery" });
    },
  });

  if (bootstrap.isPending)
    return <CenteredMessage title="Checking this device…" />;
  if (bootstrap.isError) {
    return (
      <CenteredMessage
        title="SkillBinder could not start setup"
        detail={bootstrap.error.message}
        action={<Button onClick={() => bootstrap.refetch()}>Retry</Button>}
      />
    );
  }

  const data = bootstrap.data;
  if (data.onboarding.completed) {
    return <Navigate to="/discovery" replace />;
  }

  const current = data.onboarding.step;
  const canContinue =
    data.git.prerequisite.state === "ready" && data.storage.state === "ready";
  const visibleStep = stepAfterRevalidation(current, canContinue);
  const error = update.error ?? complete.error;
  const stepNumber = onboardingSteps.indexOf(visibleStep) + 1;

  return (
    <ScrollArea className="onboarding-shell h-screen">
      <main className="grid min-h-full place-items-center p-11">
        <section className="flex min-h-[610px] w-full max-w-[820px] flex-col overflow-hidden rounded-3xl border bg-card shadow-xl">
          <header className="flex items-start justify-between gap-6 px-12 pt-10 pb-6">
            <div>
              <p className="mb-2 text-xs font-extrabold tracking-widest text-success uppercase">
                SkillBinder setup
              </p>
              <h1 className="mb-2.5 text-4xl font-normal tracking-tight">
                {stepTitle(visibleStep)}
              </h1>
              <p className="max-w-[610px] text-muted-foreground">
                {stepSubtitle(visibleStep)}
              </p>
            </div>
            <Badge variant="secondary">
              {stepNumber} / {onboardingSteps.length}
            </Badge>
          </header>
          <Progress
            value={(stepNumber / onboardingSteps.length) * 100}
            aria-label={`Setup step ${stepNumber} of ${onboardingSteps.length}`}
          />

          <div className="flex-1 px-12 py-8">
            <OnboardingStepContent
              step={visibleStep}
              data={data}
              onRetry={() => bootstrap.refetch()}
              pending={bootstrap.isFetching}
            />
          </div>

          {error ? (
            <Alert variant="destructive" className="mx-12">
              <AlertDescription>{error.message}</AlertDescription>
            </Alert>
          ) : null}
          <footer className="flex items-center justify-between border-t px-12 pt-6 pb-9">
            {visibleStep !== "prerequisites" ? (
              <Button
                variant="ghost"
                onClick={() => update.mutate(previous(current))}
                disabled={update.isPending || complete.isPending}
              >
                Back
              </Button>
            ) : (
              <span />
            )}
            {visibleStep === "ready" ? (
              <Button
                onClick={() => complete.mutate()}
                disabled={complete.isPending}
              >
                {complete.isPending
                  ? "Creating library…"
                  : "Create local library"}
              </Button>
            ) : (
              <Button
                onClick={() => update.mutate(next(current))}
                disabled={
                  (visibleStep === "prerequisites" && !canContinue) ||
                  update.isPending
                }
              >
                {visibleStep === "syncChoice"
                  ? "Continue local-only"
                  : "Continue"}
              </Button>
            )}
          </footer>
        </section>
      </main>
    </ScrollArea>
  );
}

function CenteredMessage({
  title,
  detail,
  action,
}: {
  title: string;
  detail?: string;
  action?: React.ReactNode;
}) {
  return (
    <main className="grid min-h-screen place-content-center justify-items-center gap-2.5 p-8 text-center">
      <h1>{title}</h1>
      {detail ? <p className="text-muted-foreground">{detail}</p> : null}
      {action}
    </main>
  );
}

function next(step: OnboardingStep): OnboardingStep {
  return onboardingSteps[
    Math.min(onboardingSteps.indexOf(step) + 1, onboardingSteps.length - 1)
  ];
}

function previous(step: OnboardingStep): OnboardingStep {
  return onboardingSteps[Math.max(onboardingSteps.indexOf(step) - 1, 0)];
}
