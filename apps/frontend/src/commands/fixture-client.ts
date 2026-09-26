import type {
  BindingView,
  BootstrapResponse,
  CompleteOnboardingResponse,
  DiscoveryCandidate,
  DiscoveryExclusion,
  DiscoveryLocation,
  DiscoveryProgress,
  DiscoveryWarning,
  ImportPlanResponse,
  LibraryListResponse,
  OnboardingProgress,
  OnboardingStep,
  RootView,
  ValidationSummary,
} from "@/types";
import type { DesktopClient } from "./client";
import { onboardingProgressSchema } from "./contracts";

const storageKey = "skillbinder.fixture.onboarding";
const preexistingSkillId = "fixture-existing";

const candidates = [
  {
    candidateId: "fixture-clean",
    locationId: "fixture-clean",
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
    linked: false,
    warnings: [],
  },
  {
    candidateId: "fixture-shared",
    locationId: "fixture-shared",
    displayPath: "/Users/demo/.agents/skills/research",
    slug: "research",
    name: "Research",
    description: "Research a topic with cited sources.",
    readerAgentIds: ["claude-code", "codex"],
    validation: { status: "valid" as const, messages: [] },
    duplicate: { kind: "unique" as const },
    fileCount: 4,
    totalBytes: "3072",
    linked: false,
    warnings: ["Shared location read by two agents."],
  },
  {
    candidateId: "fixture-invalid",
    locationId: "fixture-invalid",
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
    linked: false,
    warnings: [],
  },
  {
    candidateId: "fixture-blocked",
    locationId: "fixture-blocked",
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
    linked: false,
    warnings: [],
  },
  {
    candidateId: "fixture-duplicate",
    locationId: "fixture-duplicate",
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
    linked: false,
    warnings: [],
  },
  {
    candidateId: "fixture-root-link",
    locationId: "fixture-root-link",
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
    linked: true,
    warnings: [],
  },
] satisfies DiscoveryCandidate[];

const fixtureLocations: DiscoveryLocation[] = [
  {
    locationId: "fixture:loc:0",
    rootId: null,
    displayPath: "/Users/demo/.claude/skills",
    agentIds: ["claude-code"],
    agentLabels: ["Claude Code"],
    state: "scanned",
    detail: null,
    limitReached: false,
  },
  {
    locationId: "fixture:loc:1",
    rootId: null,
    displayPath: "/Users/demo/.codex/skills",
    agentIds: ["codex"],
    agentLabels: ["Codex"],
    state: "scanned",
    detail: null,
    limitReached: false,
  },
  {
    locationId: "fixture:loc:2",
    rootId: null,
    displayPath: "/Users/demo/.cursor/skills",
    agentIds: ["cursor"],
    agentLabels: ["Cursor"],
    state: "unreadable",
    detail: "Permission denied",
    limitReached: false,
  },
  {
    locationId: "fixture:loc:3",
    rootId: "fixture-project",
    displayPath: "/Users/demo/Projects/atlas",
    agentIds: [],
    agentLabels: [],
    state: "scanned",
    detail: null,
    limitReached: true,
  },
];

const fixtureExclusions: DiscoveryExclusion[] = [
  {
    name: ".git",
    reason: "vcsMetadata",
    matches: 3,
    samplePath: "/Users/demo/Projects/atlas/.git",
  },
  {
    name: "node_modules",
    reason: "dependencyVendor",
    matches: 14,
    samplePath: "/Users/demo/Projects/atlas/node_modules",
  },
  {
    name: "target",
    reason: "buildOutput",
    matches: 6,
    samplePath: "/Users/demo/Projects/atlas/crates/target",
  },
];

const fixtureWarnings: DiscoveryWarning[] = [
  {
    displayPath: "/Users/demo/Projects/atlas/locked",
    message: "Permission denied",
  },
  { displayPath: null, message: "Symlink cycle skipped at /Users/demo/.codex" },
];

const runningProgress: DiscoveryProgress = {
  rootsTotal: 2,
  rootsDone: 1,
  entriesSeen: 1240,
  candidatesFound: 3,
  currentPath: "/Users/demo/Projects/atlas/packages/api",
};

const finishedProgress: DiscoveryProgress = {
  rootsTotal: 2,
  rootsDone: 2,
  entriesSeen: 8421,
  candidatesFound: 6,
  currentPath: null,
};

/** How many `discoveryResults` polls report `running` before the fixture finishes. */
const runningPolls = 1;
const pickableFolders = [
  "/Users/demo/Projects/atlas",
  "/Users/demo/Projects/orbit",
];

let roots: RootView[] = [];
let grantSequence = 0;
let runSequence = 0;
interface FixtureGrant {
  grantId: string;
  displayPath: string;
  resolvedPath: string;
}
interface FixtureRun {
  scanId: string;
  polls: number;
  cancelled: boolean;
}
const pendingGrants = new Map<string, FixtureGrant>();
const runs = new Map<string, FixtureRun>();

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
let bindings: BindingView[] = [];

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
  async bindingsOptions(projectRootId) {
    const project = roots.find((root) => root.rootId === projectRootId);
    const base = project?.resolvedPath;
    return {
      agents: [
        {
          agentId: "codex",
          displayName: "Codex",
          detected: true,
          available: true,
          readerPath: base
            ? `${base}/.agents/skills`
            : "/Users/demo/.codex/skills",
        },
        {
          agentId: "claude-code",
          displayName: "Claude Code",
          detected: true,
          available: true,
          readerPath: base
            ? `${base}/.claude/skills`
            : "/Users/demo/.claude/skills",
        },
        {
          agentId: "cline",
          displayName: "Cline",
          detected: Boolean(base),
          available: true,
          readerPath: base
            ? `${base}/.agents/skills`
            : "/Users/demo/.agents/skills",
        },
      ],
    };
  },
  async bindingsCreate(skillIds, scope, projectRootId, agentIds) {
    const options = await this.bindingsOptions(projectRootId);
    const targets = skillIds.flatMap((skillId) => {
      const skill = library.skills.find((item) => item.skillId === skillId);
      if (!skill) throw new Error("Select a skill in the library.");
      const byPath = new Map<string, string[]>();
      for (const agent of options.agents.filter((item) =>
        agentIds.includes(item.agentId),
      )) {
        const path = `${agent.readerPath}/${skill.slug}`;
        byPath.set(path, [...(byPath.get(path) ?? []), agent.agentId]);
      }
      return [...byPath].map(([path, readerAgentIds]) => ({
        skillId,
        path,
        readerAgentIds,
        status: "installed",
      }));
    });
    const binding: BindingView = {
      bindingId: `fixture-binding-${bindings.length + 1}`,
      skillIds,
      scope,
      projectRootId,
      agentIds,
      createdAt: Date.now().toString(),
      targets,
    };
    bindings = [binding, ...bindings];
    return { binding };
  },
  async bindingsList() {
    return { bindings };
  },
  async bindingsRepair(bindingId) {
    const binding = bindings.find((item) => item.bindingId === bindingId);
    if (!binding) throw new Error("Unknown binding.");
    const repaired = {
      ...binding,
      targets: binding.targets.map((target) =>
        target.status === "missing"
          ? { ...target, status: "installed" }
          : target,
      ),
    };
    bindings = bindings.map((item) =>
      item.bindingId === bindingId ? repaired : item,
    );
    return { binding: repaired };
  },
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
  async rootsPick() {
    const displayPath = pickableFolders[grantSequence % pickableFolders.length];
    grantSequence += 1;
    const grant: FixtureGrant = {
      grantId: `fixture-grant-${grantSequence}`,
      displayPath,
      resolvedPath: displayPath,
    };
    pendingGrants.set(grant.grantId, grant);
    return { grant };
  },
  async rootsRegister(grantId, label) {
    const grant = pendingGrants.get(grantId);
    if (!grant)
      throw new Error("That folder grant expired. Pick the folder again.");
    pendingGrants.delete(grantId);
    const existing = roots.find(
      (root) => root.resolvedPath === grant.resolvedPath,
    );
    if (existing)
      throw new Error(
        `A root for that folder is already registered as ${existing.label}.`,
      );
    const root: RootView = {
      rootId: `fixture-root-${grantId}`,
      displayPath: grant.displayPath,
      resolvedPath: grant.resolvedPath,
      label:
        label?.trim() ||
        grant.displayPath.split("/").pop() ||
        grant.displayPath,
      enabled: true,
    };
    roots = [...roots, root];
    return { root };
  },
  async rootsList() {
    return { roots };
  },
  async rootsUpdate(rootId, label, enabled) {
    const existing = roots.find((root) => root.rootId === rootId);
    if (!existing) throw new Error("That root is no longer registered.");
    const root: RootView = { ...existing, label, enabled };
    roots = roots.map((candidate) =>
      candidate.rootId === rootId ? root : candidate,
    );
    return { root };
  },
  async rootsRemove(rootId) {
    roots = roots.filter((root) => root.rootId !== rootId);
    return { rootId };
  },
  async discoveryStart() {
    const live = [...runs.values()].find((run) => run.polls <= runningPolls);
    if (live) return { scanId: live.scanId };
    runSequence += 1;
    const run: FixtureRun = {
      scanId: `fixture-scan-${runSequence}`,
      polls: 0,
      cancelled: false,
    };
    runs.set(run.scanId, run);
    return { scanId: run.scanId };
  },
  async discoveryResults(scanId, offset, limit) {
    const run = runs.get(scanId);
    if (!run)
      throw new Error("That scan is no longer available. Start a new scan.");
    run.polls += 1;
    if (run.polls <= runningPolls && !run.cancelled) {
      return {
        scanId,
        phase: "running" as const,
        registryVersion: 1,
        progress: runningProgress,
        limitsReached: false,
        locations: [],
        exclusions: [],
        warnings: [],
        candidates: [],
        totalCandidates: 0,
        hiddenDuplicates: 0,
        offset,
        limit,
        failure: null,
      };
    }
    // A candidate whose payload the library already holds stays out of the list.
    const hidden = candidates.filter(
      ({ duplicate }) => duplicate.kind === "identical",
    );
    const visible = candidates.filter(
      ({ duplicate }) => duplicate.kind !== "identical",
    );
    return {
      scanId,
      phase: run.cancelled ? ("cancelled" as const) : ("finished" as const),
      registryVersion: 1,
      progress: finishedProgress,
      limitsReached: true,
      locations: fixtureLocations,
      exclusions: fixtureExclusions,
      warnings: fixtureWarnings,
      candidates: visible.slice(offset, offset + limit),
      totalCandidates: visible.length,
      hiddenDuplicates: hidden.length,
      offset,
      limit,
      failure: null,
    };
  },
  async discoveryCancel(scanId) {
    const run = runs.get(scanId);
    if (run) run.cancelled = true;
    return { scanId, accepted: true };
  },
  async discoveryCurrent() {
    const all = [...runs.values()];
    const live = all.find((run) => run.polls <= runningPolls && !run.cancelled);
    const run = live ?? all.at(-1);
    return { scanId: run?.scanId ?? null };
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
  async diagnosticsRevealLogs() {
    return {
      path: "/Users/demo/Library/Application Support/dev.skillbinder.local/logs",
    };
  },
};
