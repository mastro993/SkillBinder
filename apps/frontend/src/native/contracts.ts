import { z } from "zod";
import type {
  AppError,
  BootstrapResponse,
  CompleteOnboardingResponse,
  OnboardingProgress,
} from "@/generated";

const errorCodeSchema = z.enum([
  "validationFailed",
  "permissionDenied",
  "databaseUnavailable",
  "gitNotFound",
  "gitUnsupported",
  "gitProcessFailed",
  "gitProcessTimeout",
  "libraryBusy",
  "recoveryRequired",
  "internalError",
]);
const recoveryActionSchema = z.enum([
  "retry",
  "configureGit",
  "checkStoragePermissions",
  "openRecovery",
]);
const onboardingStepSchema = z.enum([
  "prerequisites",
  "boundaries",
  "syncChoice",
  "ready",
]);
const onboardingProgressSchema: z.ZodType<OnboardingProgress> = z
  .object({ step: onboardingStepSchema, completed: z.boolean() })
  .strict();
const prerequisiteStatusSchema = z
  .object({
    state: z.enum(["ready", "needsAttention"]),
    summary: z.string(),
    detail: z.string(),
    repairInstruction: z.string().nullable(),
  })
  .strict();
const appErrorSchema: z.ZodType<AppError> = z
  .object({
    code: errorCodeSchema,
    message: z.string(),
    retryable: z.boolean(),
    recoveryAction: recoveryActionSchema.nullable(),
    diagnosticId: z.string(),
  })
  .strict();

export const bootstrapResponseSchema: z.ZodType<BootstrapResponse> = z
  .object({
    appVersion: z.string(),
    protocolVersion: z.string(),
    capabilities: z.array(z.string()),
    git: z
      .object({
        prerequisite: prerequisiteStatusSchema,
        executable: z.string().nullable(),
        version: z.string().nullable(),
      })
      .strict(),
    storage: prerequisiteStatusSchema,
    onboarding: onboardingProgressSchema,
    libraryState: z.enum(["notCreated", "ready", "recoveryRequired"]),
    currentRevision: z.string().nullable(),
    recoverySummary: z.string().nullable(),
  })
  .strict();

export const completeOnboardingResponseSchema: z.ZodType<CompleteOnboardingResponse> =
  z
    .object({
      libraryState: z.literal("ready"),
      libraryId: z.string(),
      currentRevision: z.string(),
    })
    .strict();

export { onboardingProgressSchema };

export function parseCommandResult<T>(
  value: unknown,
  valueSchema: z.ZodType<T>,
): { ok: true; value: T } | { ok: false; error: AppError } {
  return z
    .discriminatedUnion("ok", [
      z.object({ ok: z.literal(true), value: valueSchema }).strict(),
      z.object({ ok: z.literal(false), error: appErrorSchema }).strict(),
    ])
    .parse(value);
}
