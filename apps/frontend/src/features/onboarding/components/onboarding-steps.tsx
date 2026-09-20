import {
  BookCopy,
  FolderSearch,
  GitBranch,
  HardDrive,
  LockKeyhole,
  RefreshCw,
} from "lucide-react";
import type { BootstrapResponse, OnboardingStep } from "@/generated";
import { StatusCard } from "@/components/feedback/status-card";
import { Button } from "@/components/ui/button";

export const onboardingSteps: OnboardingStep[] = [
  "prerequisites",
  "boundaries",
  "syncChoice",
  "ready",
];

export function OnboardingStepContent({
  step,
  data,
  onRetry,
  pending,
}: {
  step: OnboardingStep;
  data: BootstrapResponse;
  onRetry: () => unknown;
  pending: boolean;
}) {
  if (step === "prerequisites") {
    return (
      <div className="stack">
        <StatusCard label="Version history" status={data.git.prerequisite} />
        <StatusCard label="Local data" status={data.storage} />
        <Button variant="secondary" onClick={onRetry} disabled={pending}>
          <RefreshCw size={16} className={pending ? "spin" : undefined} />
          Recheck
        </Button>
      </div>
    );
  }
  if (step === "boundaries") return <Boundaries />;
  if (step === "syncChoice") return <SyncChoice />;
  return <Ready />;
}

export function Boundaries() {
  const points = [
    [
      BookCopy,
      "Managed copies",
      "Imports create canonical copies. Original skill folders stay unchanged.",
    ],
    [
      HardDrive,
      "Portable library",
      "Canonical skills, portable IDs, organization, and history move together in the Git library.",
    ],
    [
      HardDrive,
      "Machine-local state",
      "Device paths, settings, credentials, and deployment state stay on this machine and outside Git.",
    ],
    [
      FolderSearch,
      "Bounded discovery",
      "Global known locations and only project roots you select are scanned.",
    ],
    [
      GitBranch,
      "Local Git history",
      "Every library has version history through your supported system Git.",
    ],
    [
      LockKeyhole,
      "No account or token",
      "SkillBinder has no hosted account and stores no Git credentials.",
    ],
    [
      RefreshCw,
      "Explicit sync",
      "Remote sync stays optional and never runs in the background.",
    ],
  ] as const;
  return (
    <div className="boundary-grid">
      {points.map(([Icon, title, copy]) => (
        <article key={title} className="boundary-item">
          <Icon size={20} />
          <div>
            <h3>{title}</h3>
            <p>{copy}</p>
          </div>
        </article>
      ))}
    </div>
  );
}

function SyncChoice() {
  return (
    <div className="choice-card selected">
      <div className="choice-radio" aria-hidden="true">
        <span />
      </div>
      <div>
        <h3>Start local-only</h3>
        <p>
          Create a private local library now. Add an existing Git remote later
          from Settings.
        </p>
      </div>
      <span className="recommended">Recommended</span>
    </div>
  );
}

function Ready() {
  return (
    <div className="ready-panel">
      <div className="ready-mark">
        <LockKeyhole size={30} />
      </div>
      <h2>Ready for a local library</h2>
      <p>
        SkillBinder will initialize a Git-backed library. No network request,
        account, Node.js runtime, or remote is required.
      </p>
      <ul>
        <li>
          Canonical skill content stays under <code>skills/</code>
        </li>
        <li>
          Portable metadata stays under <code>.skillbinder/</code>
        </li>
        <li>Machine paths and settings stay outside Git</li>
      </ul>
    </div>
  );
}

export function stepTitle(step: OnboardingStep) {
  return {
    prerequisites: "Check your setup",
    boundaries: "Know what SkillBinder owns",
    syncChoice: "Choose how to begin",
    ready: "Create your library",
  }[step];
}

export function stepSubtitle(step: OnboardingStep) {
  return {
    prerequisites:
      "Required local tools must be ready before any library is created.",
    boundaries:
      "Clear lines keep source skills, managed content, and device state safe.",
    syncChoice: "Remote sync is optional. Start offline and connect one later.",
    ready: "One last review before SkillBinder writes durable state.",
  }[step];
}

export function stepAfterRevalidation(
  savedStep: OnboardingStep,
  prerequisitesReady: boolean,
): OnboardingStep {
  return prerequisitesReady ? savedStep : "prerequisites";
}
