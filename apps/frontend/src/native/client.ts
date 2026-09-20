import { invoke } from "@tauri-apps/api/core";
import type {
  BootstrapResponse,
  CompleteOnboardingResponse,
  OnboardingProgress,
  OnboardingStep,
} from "@/generated";
import {
  bootstrapResponseSchema,
  completeOnboardingResponseSchema,
  onboardingProgressSchema,
  parseCommandResult,
} from "./contracts";
import type { ZodType } from "zod";

export class NativeCommandError extends Error {
  constructor(
    message: string,
    readonly diagnosticId: string,
  ) {
    super(message);
    this.name = "NativeCommandError";
  }
}

export interface DesktopClient {
  bootstrap(): Promise<BootstrapResponse>;
  updateOnboardingProgress(step: OnboardingStep): Promise<OnboardingProgress>;
  completeLocalOnboarding(): Promise<CompleteOnboardingResponse>;
}

class TauriDesktopClient implements DesktopClient {
  bootstrap() {
    return invokeCommand("system_bootstrap", bootstrapResponseSchema);
  }

  updateOnboardingProgress(step: OnboardingStep) {
    return invokeCommand(
      "onboarding_progress_update",
      onboardingProgressSchema,
      { request: { step } },
    );
  }

  completeLocalOnboarding() {
    return invokeCommand(
      "onboarding_complete_local",
      completeOnboardingResponseSchema,
    );
  }
}

async function invokeCommand<T>(
  command: string,
  valueSchema: ZodType<T>,
  args?: Record<string, unknown>,
): Promise<T> {
  const raw = await invoke<unknown>(command, args);
  let result: ReturnType<typeof parseCommandResult<T>>;
  try {
    result = parseCommandResult(raw, valueSchema);
  } catch {
    throw new NativeCommandError(
      "Native command returned an invalid result.",
      "transport.invalid-result",
    );
  }
  if (result.ok) return result.value;
  else {
    throw new NativeCommandError(
      result.error.message,
      result.error.diagnosticId,
    );
  }
}

let clientPromise: Promise<DesktopClient> | undefined;

export function getDesktopClient(): Promise<DesktopClient> {
  clientPromise ??=
    import.meta.env.DEV &&
    import.meta.env.VITE_SKILLBINDER_FIXTURE_MODE === "true"
      ? import("./fixture-client").then(
          ({ fixtureDesktopClient }) => fixtureDesktopClient,
        )
      : Promise.resolve(new TauriDesktopClient());
  return clientPromise;
}

export const isFixtureMode =
  import.meta.env.DEV &&
  import.meta.env.VITE_SKILLBINDER_FIXTURE_MODE === "true";
