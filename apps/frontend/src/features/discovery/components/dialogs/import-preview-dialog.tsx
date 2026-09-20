import type { ImportPlanResponse } from "@/generated";
import { Button } from "@/components/ui/button";

const validationLabels = {
  valid: "Valid",
  warning: "Warning",
  invalid: "Invalid",
  blocked: "Blocked",
};

export function ImportPreviewDialog({
  plan,
  applying,
  onApply,
  onClose,
}: {
  plan: ImportPlanResponse;
  applying: boolean;
  onApply: () => void;
  onClose: () => void;
}) {
  return (
    <div className="dialog-backdrop" role="presentation">
      <dialog
        className="dialog"
        open
        aria-modal="true"
        aria-labelledby="import-preview-title"
        onKeyDown={(event) => {
          if (event.key === "Escape" && !applying) onClose();
        }}
      >
        <header className="dialog-header">
          <div>
            <p className="eyebrow">Import review</p>
            <h2 id="import-preview-title">Review import plan</h2>
          </div>
          <button
            className="dialog-close"
            type="button"
            onClick={onClose}
            aria-label="Close import review"
          >
            ×
          </button>
        </header>
        <p className="dialog-copy">
          SkillBinder will copy these sources into your local library. Originals
          stay unchanged.
        </p>
        <ul className="plan-list">
          {plan.items.map((item) => (
            <li key={item.candidateId}>
              <div>
                <strong>{item.slug}</strong>
                <span>{item.displayPath}</span>
                <span>
                  Destination skill: <code>{item.skillId}</code>
                </span>
              </div>
              <span>
                {duplicateLabel(item)} · {item.fileCount} files ·{" "}
                {item.totalBytes} bytes
              </span>
              <span>
                Validation: {validationLabels[item.validation.status]}
              </span>
              {item.validation.messages.map((message) => (
                <small key={message.code}>{message.message}</small>
              ))}
              {item.exclusions.length ? (
                <small>Warnings: {item.exclusions.join("; ")}</small>
              ) : null}
            </li>
          ))}
        </ul>
        <footer className="dialog-actions">
          <Button variant="ghost" onClick={onClose} disabled={applying}>
            Back
          </Button>
          <Button onClick={onApply} disabled={applying} autoFocus>
            {applying ? "Importing…" : "Apply import"}
          </Button>
        </footer>
      </dialog>
    </div>
  );
}

function duplicateLabel(item: ImportPlanResponse["items"][number]) {
  if (item.duplicate.kind === "identical")
    return "Attach source to the existing skill";
  if (item.duplicate.kind === "slugInUse") return "Slug already in use";
  return "New skill";
}
