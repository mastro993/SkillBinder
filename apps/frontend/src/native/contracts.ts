import { z, ZodError } from "zod";
import type {
  AppError,
  BootstrapResponse,
  CandidateDuplicate,
  CompleteOnboardingResponse,
  DiscoveryCandidate,
  DiscoveryCancelResponse,
  DiscoveryExclusion,
  DiscoveryLocation,
  DiscoveryProgress,
  DiscoveryResultsRequest,
  DiscoveryResultsResponse,
  DiscoveryStartResponse,
  DiscoveryWarning,
  ExclusionReason,
  ImportApplyResponse,
  ImportOutcome,
  ImportPlanItem,
  ImportPlanResponse,
  LibraryListResponse,
  LibrarySkill,
  OnboardingProgress,
  RootGrantView,
  RootsListResponse,
  RootsPickResponse,
  RootsRegisterRequest,
  RootsRegisterResponse,
  RootsRemoveRequest,
  RootsRemoveResponse,
  RootsUpdateRequest,
  RootsUpdateResponse,
  RootView,
  ScanPhase,
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
export const scanPhaseSchema: z.ZodType<ScanPhase> = z.enum([
  "running",
  "finished",
  "cancelled",
  "failed",
]);
export const exclusionReasonSchema: z.ZodType<ExclusionReason> = z.enum([
  "vcsMetadata",
  "dependencyVendor",
  "buildOutput",
  "cache",
  "virtualEnvironment",
  "appData",
  "mountBoundary",
]);
export const rootGrantViewSchema: z.ZodType<RootGrantView> = z
  .object({
    grantId: z.string(),
    displayPath: z.string(),
    resolvedPath: z.string(),
  })
  .strict();
export const rootViewSchema: z.ZodType<RootView> = z
  .object({
    rootId: z.string(),
    displayPath: z.string(),
    resolvedPath: z.string(),
    label: z.string(),
    enabled: z.boolean(),
  })
  .strict();
export const rootsPickResponseSchema: z.ZodType<RootsPickResponse> = z
  .object({ grant: rootGrantViewSchema.nullable() })
  .strict();
export const rootsRegisterRequestSchema: z.ZodType<RootsRegisterRequest> = z
  .object({ grantId: z.string(), label: z.string().nullable() })
  .strict();
export const rootsRegisterResponseSchema: z.ZodType<RootsRegisterResponse> = z
  .object({ root: rootViewSchema })
  .strict();
export const rootsListResponseSchema: z.ZodType<RootsListResponse> = z
  .object({ roots: z.array(rootViewSchema) })
  .strict();
export const rootsUpdateRequestSchema: z.ZodType<RootsUpdateRequest> = z
  .object({ rootId: z.string(), label: z.string(), enabled: z.boolean() })
  .strict();
export const rootsUpdateResponseSchema: z.ZodType<RootsUpdateResponse> = z
  .object({ root: rootViewSchema })
  .strict();
export const rootsRemoveRequestSchema: z.ZodType<RootsRemoveRequest> = z
  .object({ rootId: z.string() })
  .strict();
export const rootsRemoveResponseSchema: z.ZodType<RootsRemoveResponse> = z
  .object({ rootId: z.string() })
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
export const discoveryProgressSchema: z.ZodType<DiscoveryProgress> = z
  .object({
    rootsTotal: z.number(),
    rootsDone: z.number(),
    entriesSeen: z.number(),
    candidatesFound: z.number(),
    currentPath: z.string().nullable(),
  })
  .strict();
export const discoveryLocationSchema: z.ZodType<DiscoveryLocation> = z
  .object({
    locationId: z.string(),
    rootId: z.string().nullable(),
    displayPath: z.string(),
    agentIds: z.array(z.string()),
    agentLabels: z.array(z.string()),
    state: z.enum(["scanned", "missing", "unreadable"]),
    detail: z.string().nullable(),
    limitReached: z.boolean(),
  })
  .strict();
export const discoveryExclusionSchema: z.ZodType<DiscoveryExclusion> = z
  .object({
    name: z.string(),
    reason: exclusionReasonSchema,
    matches: z.number(),
    samplePath: z.string(),
  })
  .strict();
export const discoveryWarningSchema: z.ZodType<DiscoveryWarning> = z
  .object({ displayPath: z.string().nullable(), message: z.string() })
  .strict();
export const discoveryResultsRequestSchema: z.ZodType<DiscoveryResultsRequest> =
  z
    .object({
      scanId: z.string(),
      offset: z.number(),
      limit: z.number(),
    })
    .strict();
export const discoveryResultsResponseSchema: z.ZodType<DiscoveryResultsResponse> =
  z
    .object({
      scanId: z.string(),
      phase: scanPhaseSchema,
      registryVersion: z.number(),
      progress: discoveryProgressSchema,
      limitsReached: z.boolean(),
      locations: z.array(discoveryLocationSchema),
      exclusions: z.array(discoveryExclusionSchema),
      warnings: z.array(discoveryWarningSchema),
      candidates: z.array(discoveryCandidateSchema),
      totalCandidates: z.number(),
      hiddenDuplicates: z.number(),
      offset: z.number(),
      limit: z.number(),
      failure: z.string().nullable(),
    })
    .strict();
export const discoveryStartResponseSchema: z.ZodType<DiscoveryStartResponse> = z
  .object({ scanId: z.string() })
  .strict();
export const discoveryCancelRequestSchema = z
  .object({ scanId: z.string() })
  .strict();
export const discoveryCancelResponseSchema: z.ZodType<DiscoveryCancelResponse> =
  z.object({ scanId: z.string(), accepted: z.boolean() }).strict();
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

const envelopeSchema = z
  .object({
    ok: z.boolean(),
    value: z.unknown().optional(),
    error: z.unknown().optional(),
  })
  .strict();

export type CommandResultParse<T> =
  | { kind: "value"; value: T }
  | { kind: "failure"; error: AppError }
  | { kind: "malformed"; issues: string[] };

/** One line per zod issue, so a broken payload names the field that broke it. */
export function describeIssues(error: ZodError): string[] {
  return error.issues.map((issue) => {
    const path = issue.path.join(".");
    return path === "" ? issue.message : `${path}: ${issue.message}`;
  });
}

export function parseCommandResult<T>(
  value: JsonValue,
  valueSchema: z.ZodType<T>,
): CommandResultParse<T> {
  const envelope = envelopeSchema.safeParse(value);
  if (!envelope.success) {
    return { kind: "malformed", issues: describeIssues(envelope.error) };
  }
  if (envelope.data.ok) {
    const parsed = valueSchema.safeParse(envelope.data.value);
    return parsed.success
      ? { kind: "value", value: parsed.data }
      : { kind: "malformed", issues: describeIssues(parsed.error) };
  }
  const parsed = appErrorSchema.safeParse(envelope.data.error);
  return parsed.success
    ? { kind: "failure", error: parsed.data }
    : { kind: "malformed", issues: describeIssues(parsed.error) };
}
