import type {
  BootstrapResponse,
  CompleteOnboardingResponse,
  OnboardingProgress,
  OnboardingStep,
} from "@/generated";
import type { DesktopClient } from "./client";

const storageKey = "skillbinder.fixture.onboarding";

function progress(): OnboardingProgress {
  const saved = window.localStorage.getItem(storageKey);
  return saved
    ? (JSON.parse(saved) as OnboardingProgress)
    : { step: "prerequisites", completed: false };
}

function save(value: OnboardingProgress) {
  window.localStorage.setItem(storageKey, JSON.stringify(value));
  return value;
}

export const fixtureDesktopClient: DesktopClient = {
  async bootstrap(): Promise<BootstrapResponse> {
    const onboarding = progress();
    return {
      appVersion: "0.1.0-fixture",
      protocolVersion: "1",
      capabilities: [
        "localLibrary",
        "systemGit",
        "resumableOnboarding",
        "optionalRemoteSync",
      ],
      git: {
        prerequisite: {
          state: "ready",
          summary: "Git is ready",
          detail: "Fixture Git 2.50.1 is available.",
          repairInstruction: null,
        },
        executable: "/fixture/bin/git",
        version: "2.50.1",
      },
      storage: {
        state: "ready",
        summary: "Application storage is writable",
        detail: "Fixture state stays in this browser.",
        repairInstruction: null,
      },
      onboarding,
      libraryState: onboarding.completed ? "ready" : "notCreated",
      currentRevision: onboarding.completed ? "fixture-initial-revision" : null,
      recoverySummary: null,
    };
  },

  async updateOnboardingProgress(step: OnboardingStep) {
    const current = progress();
    return current.completed ? current : save({ step, completed: false });
  },

  async completeLocalOnboarding(): Promise<CompleteOnboardingResponse> {
    save({ step: "ready", completed: true });
    return {
      libraryState: "ready",
      libraryId: "fixture-library",
      currentRevision: "fixture-initial-revision",
    };
  },
};
