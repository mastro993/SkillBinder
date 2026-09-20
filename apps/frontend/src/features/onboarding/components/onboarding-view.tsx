import { useMutation, useQuery } from "@tanstack/react-query";
import { Navigate, useNavigate } from "@tanstack/react-router";
import type { OnboardingStep } from "@/generated";
import { bootstrapQuery } from "@/app/bootstrap-query";
import { queryClient } from "@/app/query";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { getDesktopClient } from "@/native/client";
import {
  OnboardingStepContent,
  onboardingSteps,
  stepAfterRevalidation,
  stepSubtitle,
  stepTitle,
} from "./onboarding-steps";

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

  return (
    <ScrollArea className="onboarding-shell">
      <main className="onboarding-frame">
        <section className="onboarding-card">
          <header className="onboarding-header">
            <div>
              <p className="eyebrow">SkillBinder setup</p>
              <h1>{stepTitle(visibleStep)}</h1>
              <p className="lead">{stepSubtitle(visibleStep)}</p>
            </div>
            <div className="step-count">
              {onboardingSteps.indexOf(visibleStep) + 1} /{" "}
              {onboardingSteps.length}
            </div>
          </header>
          <div
            className="progress-track"
            aria-label={`Setup step ${onboardingSteps.indexOf(visibleStep) + 1} of ${onboardingSteps.length}`}
          >
            <div
              style={{
                "--progress": `${((onboardingSteps.indexOf(visibleStep) + 1) / onboardingSteps.length) * 100}%`,
              }}
            />
          </div>

          <div className="onboarding-content">
            <OnboardingStepContent
              step={visibleStep}
              data={data}
              onRetry={() => bootstrap.refetch()}
              pending={bootstrap.isFetching}
            />
          </div>

          {error ? (
            <p className="inline-error" role="alert">
              {error.message}
            </p>
          ) : null}
          <footer className="onboarding-actions">
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
    <main className="centered-message">
      <h1>{title}</h1>
      {detail ? <p>{detail}</p> : null}
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
