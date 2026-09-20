import { invoke } from "@tauri-apps/api/core";
import type {
  BootstrapResponse,
  CompleteOnboardingResponse,
  DiscoveryScanResponse,
  ImportApplyRequest,
  ImportApplyResponse,
  ImportPlanResponse,
  ImportPrepareRequest,
  LibraryListResponse,
  OnboardingProgress,
  OnboardingStep,
  UpdateOnboardingProgressRequest,
} from "@/generated";
import {
  bootstrapResponseSchema,
  completeOnboardingResponseSchema,
  discoveryScanResponseSchema,
  importApplyResponseSchema,
  importApplyRequestSchema,
  importPlanResponseSchema,
  importPrepareRequestSchema,
  libraryListResponseSchema,
  onboardingProgressSchema,
  parseCommandResult,
} from "./contracts";

import type { JsonValue } from "./contracts";

import type { ZodType } from "zod";

/** The request payloads the native commands accept. */
type IpcRequest =
  | UpdateOnboardingProgressRequest
  | ImportPrepareRequest
  | ImportApplyRequest;

type IpcArguments = { request: IpcRequest };

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
  discoveryScan(): Promise<DiscoveryScanResponse>;
  importsPrepare(
    candidateIds: string[],
    allowInvalidSkills: boolean,
  ): Promise<ImportPlanResponse>;
  importsApply(planId: string): Promise<ImportApplyResponse>;
  libraryList(): Promise<LibraryListResponse>;
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

  discoveryScan() {
    return invokeCommand("discovery_scan", discoveryScanResponseSchema);
  }

  importsPrepare(candidateIds: string[], allowInvalidSkills: boolean) {
    const request = importPrepareRequestSchema.parse({
      candidateIds,
      allowInvalidSkills,
    });
    return invokeCommand("imports_prepare", importPlanResponseSchema, {
      request,
    });
  }

  importsApply(planId: string) {
    const request = importApplyRequestSchema.parse({ planId });
    return invokeCommand("imports_apply", importApplyResponseSchema, {
      request,
    });
  }

  libraryList() {
    return invokeCommand("library_list", libraryListResponseSchema);
  }
}

async function invokeCommand<T>(
  command: string,
  valueSchema: ZodType<T>,
  args?: IpcArguments,
): Promise<T> {
  const raw = await invoke<JsonValue>(command, args);
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
    (import.meta.env.VITE_SKILLBINDER_FIXTURE_MODE === "true" ||
      import.meta.env.MODE === "fixture")
      ? import("./fixture-client").then(
          ({ fixtureDesktopClient }) => fixtureDesktopClient,
        )
      : Promise.resolve(new TauriDesktopClient());
  return clientPromise;
}

export const isFixtureMode =
  import.meta.env.DEV &&
  (import.meta.env.VITE_SKILLBINDER_FIXTURE_MODE === "true" ||
    import.meta.env.MODE === "fixture");
