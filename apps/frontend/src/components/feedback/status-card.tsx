import type { PrerequisiteStatus } from "@/generated";
import { AlertCircle, CheckCircle2 } from "lucide-react";

export function StatusCard({
  label,
  status,
}: {
  label: string;
  status: PrerequisiteStatus;
}) {
  const ready = status.state === "ready";
  return (
    <article className={`status-card ${ready ? "ready" : "attention"}`}>
      <div className="status-icon" aria-hidden="true">
        {ready ? <CheckCircle2 size={20} /> : <AlertCircle size={20} />}
      </div>
      <div>
        <p className="eyebrow">{label}</p>
        <h3>{status.summary}</h3>
        <p>{status.detail}</p>
        {status.repairInstruction ? (
          <p className="repair">{status.repairInstruction}</p>
        ) : null}
      </div>
    </article>
  );
}
