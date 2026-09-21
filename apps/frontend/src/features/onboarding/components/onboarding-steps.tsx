import {
  BookCopy,
  FolderSearch,
  GitBranch,
  HardDrive,
  LockKeyhole,
  RefreshCw,
} from "lucide-react";
import type { BootstrapResponse, OnboardingStep } from "@/types";
import { StatusCard } from "@/components/feedback/status-card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import {
  Item,
  ItemContent,
  ItemDescription,
  ItemMedia,
  ItemTitle,
} from "@/components/ui/item";
import { Spinner } from "@/components/ui/spinner";

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
  onRetry: () => void;
  pending: boolean;
}) {
  if (step === "prerequisites") {
    return (
      <div className="grid gap-3.5">
        <StatusCard label="Version history" status={data.git.prerequisite} />
        <StatusCard label="Local data" status={data.storage} />
        <Button
          variant="outline"
          className="justify-self-start"
          onClick={onRetry}
          disabled={pending}
        >
          {pending ? <Spinner /> : <RefreshCw aria-hidden="true" />}
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
      "Known agent locations and known skill folders in your project roots.",
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
    <div className="grid grid-cols-2 gap-3 max-md:grid-cols-1">
      {points.map(([Icon, title, copy]) => (
        <Item key={title} variant="outline" size="sm">
          <ItemMedia variant="icon">
            <span className="text-success">
              <Icon aria-hidden="true" />
            </span>
          </ItemMedia>
          <ItemContent>
            <ItemTitle>{title}</ItemTitle>
            <ItemDescription>{copy}</ItemDescription>
          </ItemContent>
        </Item>
      ))}
    </div>
  );
}

function SyncChoice() {
  return (
    <Item variant="selected">
      <ItemMedia variant="icon">
        <LockKeyhole aria-hidden="true" />
      </ItemMedia>
      <ItemContent>
        <ItemTitle>Start local-only</ItemTitle>
        <ItemDescription>
          Create a private local library now. Add an existing Git remote later
          from Settings.
        </ItemDescription>
      </ItemContent>
      <Badge variant="success">Recommended</Badge>
    </Item>
  );
}

function Ready() {
  return (
    <Empty className="mx-auto max-w-[590px]">
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <LockKeyhole aria-hidden="true" />
        </EmptyMedia>
        <EmptyTitle>Ready for a local library</EmptyTitle>
        <EmptyDescription>
          SkillBinder will initialize a Git-backed library. No network request,
          account, Node.js runtime, or remote is required.
        </EmptyDescription>
      </EmptyHeader>
      <EmptyContent>
        <ul className="grid list-inside list-disc gap-2 text-left">
          <li>
            Canonical skill content stays under <code>skills/</code>
          </li>
          <li>
            Portable metadata stays under <code>.skillbinder/</code>
          </li>
          <li>Machine paths and settings stay outside Git</li>
        </ul>
      </EmptyContent>
    </Empty>
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
