import type {
  BootstrapResponse,
  CompleteOnboardingResponse,
  DiscoveryScanResponse,
  ImportPlanResponse,
  LibraryListResponse,
  OnboardingProgress,
  OnboardingStep,
  ValidationSummary,
} from "@/generated";
import type { DesktopClient } from "./client";
import { onboardingProgressSchema } from "./contracts";

const storageKey = "skillbinder.fixture.onboarding";
const preexistingSkillId = "fixture-existing";
const candidates = [
  {
    candidateId: "fixture-clean",
    displayPath: "/Users/demo/.claude/skills/review",
    slug: "review",
    name: "Review",
    description: "Review code for correctness.",
    readerAgentIds: ["claude-code"],
    validation: {
      status: "valid" as const,
      messages: [],
    },
    duplicate: { kind: "unique" as const },
    fileCount: 3,
    totalBytes: "2048",
    warnings: [],
  },
  {
    candidateId: "fixture-shared",
    displayPath: "/Users/demo/.agents/skills/research",
    slug: "research",
    name: "Research",
    description: "Research a topic with cited sources.",
    readerAgentIds: ["claude-code", "codex"],
    validation: { status: "valid" as const, messages: [] },
    duplicate: { kind: "unique" as const },
    fileCount: 4,
    totalBytes: "3072",
    warnings: ["Shared location read by two agents."],
  },
  {
    candidateId: "fixture-invalid",
    displayPath: "/Users/demo/.codex/skills/outline",
    slug: "outline",
    name: "Outline",
    description: null,
    readerAgentIds: ["codex"],
    validation: {
      status: "invalid" as const,
      messages: [
        {
          code: "descriptionMissing" as const,
          message: "Description is missing.",
        },
      ],
    },
    duplicate: { kind: "unique" as const },
    fileCount: 1,
    totalBytes: "512",
    warnings: [],
  },
  {
    candidateId: "fixture-blocked",
    displayPath: "/Users/demo/.cursor/skills/unsafe",
    slug: "unsafe",
    name: "Unsafe",
    description: "Blocked skill.",
    readerAgentIds: ["cursor"],
    validation: {
      status: "blocked" as const,
      messages: [
        {
          code: "unsafeEntryPath" as const,
          message: "Entry path escapes skill root.",
        },
      ],
    },
    duplicate: { kind: "unique" as const },
    fileCount: 2,
    totalBytes: "1024",
    warnings: [],
  },
  {
    candidateId: "fixture-duplicate",
    displayPath: "/Users/demo/.claude/skills/existing",
    slug: "existing",
    name: "Existing",
    description: "Already imported.",
    readerAgentIds: ["claude-code"],
    validation: { status: "valid" as const, messages: [] },
    duplicate: {
      kind: "identical" as const,
      skillId: preexistingSkillId,
      slug: "existing",
    },
    fileCount: 2,
    totalBytes: "1536",
    warnings: [],
  },
  {
    candidateId: "fixture-root-link",
    displayPath: "/Users/demo/shared/docs",
    slug: "docs",
    name: "Docs",
    description: "Write clear docs.",
    readerAgentIds: ["gemini"],
    validation: {
      status: "warning" as const,
      messages: [
        {
          code: "vcsMetadataExcluded" as const,
          message: "Version-control metadata excluded.",
        },
      ],
    },
    duplicate: { kind: "unique" as const },
    fileCount: 5,
    totalBytes: "4096",
    warnings: [],
  },
] satisfies DiscoveryScanResponse["candidates"];

const locationState = (
  displayPath: string,
  agentIds: string[],
  agentLabels: string[],
  state: "scanned" | "missing" | "unreadable",
  detail: string | null = null,
) => ({ displayPath, agentIds, agentLabels, state, detail });

const scan: DiscoveryScanResponse = {
  registryVersion: 1,
  locations: [
    locationState(
      "/Users/demo/.claude/skills",
      ["claude-code"],
      ["Claude Code"],
      "scanned",
    ),
    locationState("/Users/demo/.codex/skills", ["codex"], ["Codex"], "scanned"),
    locationState(
      "/Users/demo/.cursor/skills",
      ["cursor"],
      ["Cursor"],
      "unreadable",
      "Permission denied",
    ),
    locationState(
      "/Users/demo/.openclaw/skills",
      ["openclaw"],
      ["OpenClaw"],
      "missing",
      "Directory does not exist",
    ),
  ],
  candidates,
};

const existingValidation: ValidationSummary = { status: "valid", messages: [] };
let library: LibraryListResponse = {
  libraryRevision: "fixture-initial-revision",
  hasUncommittedChanges: false,
  skills: [
    {
      skillId: preexistingSkillId,
      slug: "existing",
      displayName: "Existing",
      description: "Already imported.",
      validation: existingValidation,
      fileCount: 2,
      totalBytes: "1536",
      sources: [
        {
          displayPath: "/Users/demo/.claude/skills/existing",
          readerAgentIds: ["claude-code"],
        },
      ],
    },
  ],
};
let plan: ImportPlanResponse | null = null;

function progress(): OnboardingProgress {
  const saved = window.localStorage.getItem(storageKey);
  if (!saved) return { step: "prerequisites", completed: false };
  const parsed = onboardingProgressSchema.safeParse(JSON.parse(saved));
  return parsed.success
    ? parsed.data
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
  async discoveryScan() {
    return scan;
  },
  async importsPrepare(candidateIds, allowInvalidSkills) {
    const selected = candidates.filter(({ candidateId }) =>
      candidateIds.includes(candidateId),
    );
    const blocked = selected.find(
      ({ validation }) => validation.status === "blocked",
    );
    if (blocked)
      throw new Error(`Cannot import ${blocked.slug}: skill is blocked.`);
    const invalid = selected.find(
      ({ validation }) => validation.status === "invalid",
    );
    if (invalid && !allowInvalidSkills)
      throw new Error("Confirm invalid skills before preparing import.");
    plan = {
      planId: `fixture-plan-${Date.now()}`,
      expiresAt: new Date(Date.now() + 300_000).toISOString(),
      libraryRevision: library.libraryRevision,
      items: selected.map((candidate) => ({
        candidateId: candidate.candidateId,
        displayPath: candidate.displayPath,
        slug: candidate.slug,
        skillId:
          candidate.duplicate.kind === "identical"
            ? candidate.duplicate.skillId
            : `skill-${candidate.candidateId}`,
        outcome:
          candidate.duplicate.kind === "identical"
            ? {
                kind: "attachObservation" as const,
                skillId: candidate.duplicate.skillId,
              }
            : { kind: "newSkill" as const },
        validation: candidate.validation,
        duplicate: candidate.duplicate,
        fileCount: candidate.fileCount,
        totalBytes: candidate.totalBytes,
        exclusions: candidate.warnings,
      })),
    };
    return plan;
  },
  async importsApply(planId) {
    if (!plan || plan.planId !== planId)
      throw new Error("Import plan is stale. Rescan and try again.");
    const imported = plan.items.map((item) => ({
      skillId: item.skillId,
      slug: item.slug,
      displayPath: item.displayPath,
      outcome: item.outcome,
      fileCount: item.fileCount,
      totalBytes: item.totalBytes,
    }));
    for (const item of plan.items) {
      const source = {
        displayPath: item.displayPath,
        readerAgentIds:
          candidates.find(({ candidateId }) => candidateId === item.candidateId)
            ?.readerAgentIds ?? [],
      };
      const existing = library.skills.find(
        ({ skillId }) => skillId === item.skillId,
      );
      if (existing) existing.sources.push(source);
      else {
        const candidate = candidates.find(
          ({ candidateId }) => candidateId === item.candidateId,
        );
        library.skills.push({
          skillId: item.skillId,
          slug: item.slug,
          displayName: candidate?.name ?? item.slug,
          description: candidate?.description ?? null,
          validation: item.validation,
          fileCount: item.fileCount,
          totalBytes: item.totalBytes,
          sources: [source],
        });
      }
    }
    library = {
      ...library,
      libraryRevision: `fixture-revision-${Date.now()}`,
      hasUncommittedChanges: true,
    };
    plan = null;
    return { planId, imported, libraryRevision: library.libraryRevision };
  },
  async libraryList() {
    return library;
  },
};
