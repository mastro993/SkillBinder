import { invoke } from "@tauri-apps/api/core";
import type {
  BootstrapResponse,
  CompleteOnboardingResponse,
  DiscoveryCancelRequest,
  DiscoveryCancelResponse,
  DiscoveryResultsRequest,
  DiscoveryResultsResponse,
  DiscoveryStartResponse,
  ImportApplyRequest,
  ImportApplyResponse,
  ImportPlanResponse,
  ImportPrepareRequest,
  LibraryListResponse,
  OnboardingProgress,
  OnboardingStep,
  RootsListResponse,
  RootsPickResponse,
  RootsRegisterRequest,
  RootsRegisterResponse,
  RootsRemoveRequest,
  RootsRemoveResponse,
  RootsUpdateRequest,
  RootsUpdateResponse,
  UpdateOnboardingProgressRequest,
} from "@/generated";
import {
  bootstrapResponseSchema,
  completeOnboardingResponseSchema,
  discoveryCancelResponseSchema,
  discoveryResultsResponseSchema,
  discoveryStartResponseSchema,
  importApplyResponseSchema,
  importPlanResponseSchema,
  libraryListResponseSchema,
  onboardingProgressSchema,
  parseCommandResult,
  rootsListResponseSchema,
  rootsPickResponseSchema,
  rootsRegisterResponseSchema,
  rootsRemoveResponseSchema,
  rootsUpdateResponseSchema,
} from "./contracts";

import type { JsonValue } from "./contracts";

import { z, type ZodType } from "zod";

/** The request payloads the native commands accept. */
type IpcRequest =
  | UpdateOnboardingProgressRequest
  | RootsRegisterRequest
  | RootsUpdateRequest
  | RootsRemoveRequest
  | DiscoveryResultsRequest
  | DiscoveryCancelRequest
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
  rootsPick(): Promise<RootsPickResponse>;
  rootsRegister(
    grantId: string,
    label: string | null,
  ): Promise<RootsRegisterResponse>;
  rootsList(): Promise<RootsListResponse>;
  rootsUpdate(
    rootId: string,
    label: string,
    enabled: boolean,
  ): Promise<RootsUpdateResponse>;
  rootsRemove(rootId: string): Promise<RootsRemoveResponse>;
  discoveryStart(): Promise<DiscoveryStartResponse>;
  discoveryResults(
    scanId: string,
    offset: number,
    limit: number,
  ): Promise<DiscoveryResultsResponse>;
  discoveryCancel(scanId: string): Promise<DiscoveryCancelResponse>;
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

  rootsPick() {
    return invokeCommand("roots_pick", rootsPickResponseSchema);
  }

  rootsRegister(grantId: string, label: string | null) {
    return invokeCommand("roots_register", rootsRegisterResponseSchema, {
      request: { grantId, label },
    });
  }

  rootsList() {
    return invokeCommand("roots_list", rootsListResponseSchema);
  }

  rootsUpdate(rootId: string, label: string, enabled: boolean) {
    return invokeCommand("roots_update", rootsUpdateResponseSchema, {
      request: { rootId, label, enabled },
    });
  }

  rootsRemove(rootId: string) {
    return invokeCommand("roots_remove", rootsRemoveResponseSchema, {
      request: { rootId },
    });
  }

  discoveryStart() {
    return invokeCommand("discovery_start", discoveryStartResponseSchema);
  }

  discoveryResults(scanId: string, offset: number, limit: number) {
    return invokeCommand("discovery_results", discoveryResultsResponseSchema, {
      request: { scanId, offset, limit },
    });
  }

  discoveryCancel(scanId: string) {
    return invokeCommand("discovery_cancel", discoveryCancelResponseSchema, {
      request: { scanId },
    });
  }

  importsPrepare(candidateIds: string[], allowInvalidSkills: boolean) {
    return invokeCommand("imports_prepare", importPlanResponseSchema, {
      request: { candidateIds, allowInvalidSkills },
    });
  }

  importsApply(planId: string) {
    return invokeCommand("imports_apply", importApplyResponseSchema, {
      request: { planId },
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
  let raw: JsonValue;
  try {
    raw = await invoke<JsonValue>(command, args);
  } catch (cause: unknown) {
    throw new NativeCommandError(
      describeRejection(cause),
      "transport.rejected",
    );
  }
  const parsed = parseCommandResult(raw, valueSchema);
  if (parsed.kind === "value") return parsed.value;
  if (parsed.kind === "failure") {
    throw new NativeCommandError(
      parsed.error.message,
      parsed.error.diagnosticId,
    );
  }
  throw new NativeCommandError(
    `The ${command} command answered with an invalid result. ${parsed.issues.join("; ")}`,
    "transport.invalid-result",
  );
}

const textRejection = z.string();
const messageRejection = z.object({ message: z.string() });

/** A rejected call carries a message from the shell, or nothing this side can read. */
function describeRejection(cause: unknown): string {
  const text = textRejection.safeParse(cause);
  if (text.success) return text.data;
  const object = messageRejection.safeParse(cause);
  if (object.success) return object.data.message;
  return "The native command failed before it could answer.";
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
