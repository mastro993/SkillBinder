import type { PrerequisiteStatus } from "@/types";
import {
  AlertCircleIcon,
  CheckmarkCircle02Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";

export function StatusCard({
  label,
  status,
}: {
  label: string;
  status: PrerequisiteStatus;
}) {
  const ready = status.state === "ready";
  return (
    <Alert variant={ready ? "success" : "warning"} role="note">
      {ready ? (
        <HugeiconsIcon icon={CheckmarkCircle02Icon} aria-hidden="true" />
      ) : (
        <HugeiconsIcon icon={AlertCircleIcon} aria-hidden="true" />
      )}
      <div className="flex flex-col gap-0.5">
        <p className="text-xs font-extrabold tracking-widest uppercase">
          {label}
        </p>
        <AlertTitle>{status.summary}</AlertTitle>
        <AlertDescription>
          {status.detail}
          {status.repairInstruction ? (
            <span className="block pt-2 font-semibold">
              {status.repairInstruction}
            </span>
          ) : null}
        </AlertDescription>
      </div>
    </Alert>
  );
}
