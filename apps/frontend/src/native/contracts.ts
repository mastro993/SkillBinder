import { z } from "zod";
import type {
  AppError,
  BootstrapResponse,
  CandidateDuplicate,
  CompleteOnboardingResponse,
  DiscoveryCandidate,
  DiscoveryScanResponse,
  GlobalLocation,
  ImportApplyResponse,
  ImportOutcome,
  ImportPlanItem,
  ImportPlanResponse,
  LibraryListResponse,
  LibrarySkill,
  OnboardingProgress,
  SkillSource,
  ValidationMessage,
  ValidationSummary,
} from "@/generated";

const errorCodeSchema = z.enum([
  "validationFailed",
  "unsupportedSkill",
  "invalidPath",
  "sourceChanged",
  "stalePlan",
  "limitExceeded",
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
  "rescanDiscovery",
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
const validationMessageSchema: z.ZodType<ValidationMessage> = z
  .object({
    code: z.enum([
      "missingSkillFile",
      "invalidFrontmatter",
      "unsupportedYaml",
      "invalidUtf8",
      "nameMissing",
      "nameMismatch",
      "nameTooLong",
      "nameHyphenRule",
      "descriptionMissing",
      "descriptionTooLong",
      "unsafeEntryPath",
      "reservedEntryName",
      "caseCollision",
      "unsupportedEntryType",
      "externalSymlink",
      "linkCycle",
      "vcsMetadataExcluded",
      "pluginManifest",
      "payloadLimitExceeded",
      "fileLimitExceeded",
      "indexNotBuilt",
    ]),
    message: z.string(),
  })
  .strict();
const validationSummarySchema: z.ZodType<ValidationSummary> = z
  .object({
    status: z.enum(["valid", "warning", "invalid", "blocked"]),
    messages: z.array(validationMessageSchema),
  })
  .strict();
const candidateDuplicateSchema: z.ZodType<CandidateDuplicate> =
  z.discriminatedUnion("kind", [
    z.object({ kind: z.literal("unique") }).strict(),
    z
      .object({
        kind: z.literal("identical"),
        skillId: z.string(),
        slug: z.string(),
      })
      .strict(),
    z
      .object({
        kind: z.literal("slugInUse"),
        skillId: z.string(),
        slug: z.string(),
      })
      .strict(),
  ]);
const globalLocationSchema: z.ZodType<GlobalLocation> = z
  .object({
    displayPath: z.string(),
    agentIds: z.array(z.string()),
    agentLabels: z.array(z.string()),
    state: z.enum(["scanned", "missing", "unreadable"]),
    detail: z.string().nullable(),
  })
  .strict();
const discoveryCandidateSchema: z.ZodType<DiscoveryCandidate> = z
  .object({
    candidateId: z.string(),
    locationId: z.string(),
    displayPath: z.string(),
    slug: z.string(),
    name: z.string().nullable(),
    description: z.string().nullable(),
    readerAgentIds: z.array(z.string()),
    validation: validationSummarySchema,
    duplicate: candidateDuplicateSchema,
    fileCount: z.number(),
    totalBytes: z.string(),
    linked: z.boolean(),
    warnings: z.array(z.string()),
  })
  .strict();
export const discoveryScanResponseSchema: z.ZodType<DiscoveryScanResponse> = z
  .object({
    registryVersion: z.number(),
    locations: z.array(globalLocationSchema),
    candidates: z.array(discoveryCandidateSchema),
  })
  .strict();
const importOutcomeSchema: z.ZodType<ImportOutcome> = z.discriminatedUnion(
  "kind",
  [
    z.object({ kind: z.literal("newSkill") }).strict(),
    z
      .object({ kind: z.literal("attachObservation"), skillId: z.string() })
      .strict(),
  ],
);
const importPlanItemSchema: z.ZodType<ImportPlanItem> = z
  .object({
    candidateId: z.string(),
    displayPath: z.string(),
    slug: z.string(),
    skillId: z.string(),
    outcome: importOutcomeSchema,
    validation: validationSummarySchema,
    duplicate: candidateDuplicateSchema,
    fileCount: z.number(),
    totalBytes: z.string(),
    exclusions: z.array(z.string()),
  })
  .strict();
export const importPlanResponseSchema: z.ZodType<ImportPlanResponse> = z
  .object({
    planId: z.string(),
    expiresAt: z.string(),
    libraryRevision: z.string().nullable(),
    items: z.array(importPlanItemSchema),
  })
  .strict();
const skillSourceSchema: z.ZodType<SkillSource> = z
  .object({
    displayPath: z.string(),
    readerAgentIds: z.array(z.string()),
  })
  .strict();
const librarySkillSchema: z.ZodType<LibrarySkill> = z
  .object({
    skillId: z.string(),
    slug: z.string(),
    displayName: z.string().nullable(),
    description: z.string().nullable(),
    validation: validationSummarySchema,
    fileCount: z.number(),
    totalBytes: z.string(),
    sources: z.array(skillSourceSchema),
  })
  .strict();
export const libraryListResponseSchema: z.ZodType<LibraryListResponse> = z
  .object({
    libraryRevision: z.string().nullable(),
    hasUncommittedChanges: z.boolean(),
    skills: z.array(librarySkillSchema),
  })
  .strict();
export const importApplyResponseSchema: z.ZodType<ImportApplyResponse> = z
  .object({
    planId: z.string(),
    imported: z.array(
      z
        .object({
          skillId: z.string(),
          slug: z.string(),
          displayPath: z.string(),
          outcome: importOutcomeSchema,
          fileCount: z.number(),
          totalBytes: z.string(),
        })
        .strict(),
    ),
    libraryRevision: z.string().nullable(),
  })
  .strict();
export const importPrepareRequestSchema = z
  .object({
    candidateIds: z.array(z.string()),
    allowInvalidSkills: z.boolean(),
  })
  .strict();
export const importApplyRequestSchema = z
  .object({ planId: z.string() })
  .strict();

export { onboardingProgressSchema };

/** A value that can cross the JSON IPC boundary. */
export type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue };

export function parseCommandResult<T>(
  value: JsonValue,
  valueSchema: z.ZodType<T>,
): { ok: true; value: T } | { ok: false; error: AppError } {
  return z
    .discriminatedUnion("ok", [
      z.object({ ok: z.literal(true), value: valueSchema }).strict(),
      z.object({ ok: z.literal(false), error: appErrorSchema }).strict(),
    ])
    .parse(value);
}
