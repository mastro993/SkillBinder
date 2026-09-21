import type { PrerequisiteStatus } from "@/types";
import { AlertCircle, CheckCircle2 } from "lucide-react";
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
        <CheckCircle2 aria-hidden="true" />
      ) : (
        <AlertCircle aria-hidden="true" />
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
