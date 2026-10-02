import type {
  BootstrapResponse,
  CompleteOnboardingResponse,
  DiscoveryCandidate,
  DiscoveryProgress,
  GitSyncConnectRequest,
  GitSyncStatus,
  ImportPlanResponse,
  LibraryListResponse,
  LibrarySkillPreviewRequest,
  LibraryResolveConflictRequest,
  OnboardingProgress,
  OrganizationChangeRequest,
  OrganizationDeletePreviewRequest,
  OnboardingStep,
  RootView,
  ValidationSummary,
} from "@/types";
import type { DesktopClient } from "./client";
import {
  libraryListResponseSchema,
  onboardingProgressSchema,
} from "./contracts";

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
    readerAgentLabels: ["Claude Code"],
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
    displayPath: "/Users/demo/.agents/skills/research",
    slug: "research",
    name: "Research",
    description: "Research a topic with cited sources.",
    readerAgentIds: ["claude-code", "codex"],
    readerAgentLabels: ["Claude Code", "Codex"],
    validation: { status: "valid" as const, messages: [] },
    duplicate: { kind: "unique" as const },
    fileCount: 4,
    totalBytes: "3072",
    linked: false,
    warnings: ["Shared location read by two agents."],
  },
  {
    candidateId: "fixture-invalid",
    displayPath: "/Users/demo/.codex/skills/outline",
    slug: "outline",
    name: "Outline",
    description: null,
    readerAgentIds: ["codex"],
    readerAgentLabels: ["Codex"],
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
    displayPath: "/Users/demo/.cursor/skills/unsafe",
    slug: "unsafe",
    name: "Unsafe",
    description: "Blocked skill.",
    readerAgentIds: ["cursor"],
    readerAgentLabels: ["Cursor"],
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
    displayPath: "/Users/demo/.claude/skills/existing",
    slug: "existing",
    name: "Existing",
    description: "Already imported.",
    readerAgentIds: ["claude-code"],
    readerAgentLabels: ["Claude Code"],
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
    displayPath: "/Users/demo/shared/docs",
    slug: "docs",
    name: "Docs",
    description: "Write clear docs.",
    readerAgentIds: ["gemini"],
    readerAgentLabels: ["Gemini"],
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
  pendingResolution: false,
  organizationRevision: "fixture-organization-0",
  folders: [],
  tags: [],
  skills: [
    {
      skillId: "00000000-0000-4000-8000-000000000001",
      slug: "existing",
      displayName: "Existing",
      description: "Already imported.",
      folderId: null,
      tagIds: [],
      validation: existingValidation,
      fileCount: 2,
      totalBytes: "1536",
      sources: [
        {
          displayPath: "/Users/demo/.claude/skills/existing",
          readerAgentIds: ["claude-code"],
        },
      ],
      digest:
        "sha256:1111111111111111111111111111111111111111111111111111111111111111",
      payloadDirectory: "existing",
    },
    {
      skillId: "00000000-0000-4000-8000-000000000002",
      slug: "existing",
      displayName: null,
      folderId: null,
      tagIds: [],
      description: "An older copy of the same name.",
      validation: existingValidation,
      fileCount: 1,
      totalBytes: "640",
      sources: [],
      digest:
        "sha256:2222222222222222222222222222222222222222222222222222222222222222",
      payloadDirectory: "existing-00000000",
    },
  ],
};
const organizationStorageKey = "skillbinder.fixture.organization";
const savedOrganization = window.localStorage.getItem(organizationStorageKey);
if (savedOrganization) {
  const saved = libraryListResponseSchema.safeParse(
    JSON.parse(savedOrganization),
  );
  if (saved.success) library = saved.data;
}
let organizationCounter = 0;
function saveOrganization() {
  organizationCounter += 1;
  library.organizationRevision = `fixture-organization-${organizationCounter}`;
  library.hasUncommittedChanges = true;
  window.localStorage.setItem(organizationStorageKey, JSON.stringify(library));
}
function previewDelete(request: OrganizationDeletePreviewRequest) {
  if (request.entity === "folder") {
    if (!library.folders.some((folder) => folder.id === request.id))
      throw new Error("Folder not found.");
    return {
      affectedSkills: library.skills.filter(
        (skill) => skill.folderId === request.id,
      ).length,
      organizationRevision: library.organizationRevision,
    };
  }
  if (request.entity === "tag") {
    if (!library.tags.some((tag) => tag.id === request.id))
      throw new Error("Tag not found.");
    return {
      affectedSkills: library.skills.filter((skill) =>
        skill.tagIds.includes(request.id),
      ).length,
      organizationRevision: library.organizationRevision,
    };
  }
  throw new Error("Unknown item type.");
}
function changeOrganization(request: OrganizationChangeRequest) {
  const { change, expectedRevision } = request;
  if (
    (change.kind === "deleteFolder" || change.kind === "deleteTag") &&
    expectedRevision !== library.organizationRevision
  )
    throw new Error("Organization changed. Review it again before deleting.");
  const name = "name" in change ? change.name.trim() : "";
  if ("name" in change && (!name || name.length > 100))
    throw new Error("Name must contain 1–100 characters.");
  const key = (value: string) => value.normalize("NFKC").toLocaleLowerCase();
  if (change.kind === "createFolder" || change.kind === "updateFolder") {
    if (
      library.folders.some(
        (folder) =>
          key(folder.name) === key(name) &&
          (change.kind !== "updateFolder" || folder.id !== change.id),
      )
    )
      throw new Error("A folder with this name already exists.");
    if (change.kind === "createFolder")
      library.folders.push({ id: crypto.randomUUID(), name });
    else {
      const folder = library.folders.find((item) => item.id === change.id);
      if (!folder) throw new Error("Folder not found.");
      folder.name = name;
    }
  } else if (change.kind === "deleteFolder") {
    previewDelete({ entity: "folder", id: change.id });
    library.folders = library.folders.filter((item) => item.id !== change.id);
    library.skills = library.skills.map((skill) =>
      skill.folderId === change.id ? { ...skill, folderId: null } : skill,
    );
  } else if (change.kind === "createTag" || change.kind === "renameTag") {
    if (
      library.tags.some(
        (tag) =>
          key(tag.name) === key(name) &&
          (change.kind !== "renameTag" || tag.id !== change.id),
      )
    )
      throw new Error("A tag with this name already exists.");
    if (change.kind === "createTag")
      library.tags.push({ id: crypto.randomUUID(), name });
    else {
      const tag = library.tags.find((item) => item.id === change.id);
      if (!tag) throw new Error("Tag not found.");
      tag.name = name;
    }
  } else if (change.kind === "deleteTag") {
    previewDelete({ entity: "tag", id: change.id });
    library.tags = library.tags.filter((item) => item.id !== change.id);
    library.skills = library.skills.map((skill) => ({
      ...skill,
      tagIds: skill.tagIds.filter((id) => id !== change.id),
    }));
  } else {
    if (
      !change.skillIds.length ||
      change.skillIds.some(
        (id) => !library.skills.some((skill) => skill.skillId === id),
      )
    )
      throw new Error("Skill not found.");
    if (
      change.setFolder &&
      change.folderId &&
      !library.folders.some((folder) => folder.id === change.folderId)
    )
      throw new Error("Folder not found.");
    if (
      change.addTagIds.some((id) => !library.tags.some((tag) => tag.id === id))
    )
      throw new Error("Tag not found.");
    library.skills = library.skills.map((skill) =>
      change.skillIds.includes(skill.skillId)
        ? {
            ...skill,
            folderId: change.setFolder ? change.folderId : skill.folderId,
            tagIds: [...new Set([...skill.tagIds, ...change.addTagIds])]
              .filter((id) => !change.removeTagIds.includes(id))
              .sort(),
          }
        : skill,
    );
  }
  saveOrganization();
  return { organizationRevision: library.organizationRevision };
}
let plan: ImportPlanResponse | null = null;
let gitSync: GitSyncStatus = {
  state: "notConfigured",
  remote: null,
  branch: null,
  localRevision: "fixture-initial-revision",
  remoteRevision: null,
  ahead: 0,
  behind: 0,
  hasLocalChanges: false,
};

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
          folderId: null,
          tagIds: [],
          validation: item.validation,
          fileCount: item.fileCount,
          totalBytes: item.totalBytes,
          sources: [source],
          digest: `sha256:${item.skillId.slice(0, 8).padEnd(8, "0")}${"0".repeat(56)}`,
          payloadDirectory: item.slug,
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
    return structuredClone(library);
  },
  async librarySkillPreview(request: LibrarySkillPreviewRequest) {
    const older = request.skillId.endsWith("0002");
    const path = request.path ?? "SKILL.md";
    const files = older
      ? ["SKILL.md"]
      : ["SKILL.md", "references/checklist.md"];
    const content =
      path === "SKILL.md"
        ? older
          ? "---\nname: existing\ndescription: Older guidance.\n---\n\nUse the older review process."
          : "---\nname: existing\ndescription: Current guidance.\n---\n\nReview the change and consult references/checklist.md."
        : path === "references/checklist.md" && !older
          ? "# Review checklist\n\n- Check behavior.\n- Check tests."
          : null;
    return {
      skillId: request.skillId,
      lastEditedAt: older ? 1_740_000_000 : 1_760_000_000,
      files,
      path,
      content,
      unavailableReason: content === null ? "File not available." : null,
    };
  },
  async libraryOrganizationChange(request) {
    return changeOrganization(request);
  },
  async libraryOrganizationPreviewDelete(request) {
    return previewDelete(request);
  },
  async libraryResolveConflict(request: LibraryResolveConflictRequest) {
    const removedSkillIds = request.expectedSkillIds.filter(
      (skillId) => skillId !== request.keepSkillId,
    );
    library = {
      ...library,
      libraryRevision: `fixture-revision-${Date.now()}`,
      hasUncommittedChanges: true,
      skills: library.skills
        .filter((skill) => !removedSkillIds.includes(skill.skillId))
        .map((skill) =>
          skill.skillId === request.keepSkillId
            ? { ...skill, slug: request.slug, payloadDirectory: request.slug }
            : skill,
        ),
    };
    return {
      slug: request.slug,
      keptSkillId: request.keepSkillId,
      removedSkillIds,
      payloadDirectory: request.slug,
      libraryRevision: library.libraryRevision,
      hasUncommittedChanges: true,
    };
  },
  async diagnosticsRevealLogs() {
    return {
      path: "/Users/demo/.skillbinder/logs",
    };
  },
  async gitSyncStatus() {
    return gitSync;
  },
  async gitSyncConnect(request: GitSyncConnectRequest) {
    gitSync = {
      ...gitSync,
      state: "needsPush",
      remote: request.remote,
      branch: request.branch,
    };
    return gitSync;
  },
  async gitSyncRefresh() {
    return gitSync;
  },
  async gitSyncPull() {
    gitSync = {
      ...gitSync,
      state: "synced",
      remoteRevision: gitSync.localRevision,
    };
    return gitSync;
  },
  async gitSyncPush() {
    gitSync = {
      ...gitSync,
      state: "synced",
      remoteRevision: gitSync.localRevision,
      ahead: 0,
      behind: 0,
      hasLocalChanges: false,
    };
    library = { ...library, hasUncommittedChanges: false };
    return gitSync;
  },
  async gitSync() {
    return fixtureDesktopClient.gitSyncPush();
  },
  async gitSyncDisconnect() {
    gitSync = {
      ...gitSync,
      state: "notConfigured",
      remote: null,
      branch: null,
      remoteRevision: null,
      ahead: 0,
      behind: 0,
    };
    return gitSync;
  },
};
