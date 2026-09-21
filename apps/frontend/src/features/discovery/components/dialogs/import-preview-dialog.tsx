import { Dialog } from "@base-ui/react/dialog";
import type { ImportPlanResponse } from "@/generated";
import { Button } from "@/components/ui/button";

const validationLabels = {
  valid: "Valid",
  warning: "Warning",
  invalid: "Invalid",
  blocked: "Blocked",
};

/**
 * A modal review of what an import will copy. Escape, the backdrop, and the close button all ask to
 * dismiss; an import in flight refuses, because the plan is single-use and half-closed is worse than
 * open.
 */
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
    <Dialog.Root
      open
      modal
      onOpenChange={(open) => {
        if (!open && !applying) onClose();
      }}
    >
      <Dialog.Portal>
        <Dialog.Backdrop className="dialog-backdrop" />
        <Dialog.Popup className="dialog">
          <header className="dialog-header">
            <div>
              <p className="eyebrow">Import review</p>
              <Dialog.Title>Review import plan</Dialog.Title>
            </div>
            <Dialog.Close
              className="dialog-close"
              aria-label="Close import review"
              disabled={applying}
            >
              ×
            </Dialog.Close>
          </header>
          <Dialog.Description className="dialog-copy">
            SkillBinder will copy these sources into your local library.
            Originals stay unchanged.
          </Dialog.Description>
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
            <Button onClick={onApply} disabled={applying}>
              {applying ? "Importing…" : "Apply import"}
            </Button>
          </footer>
        </Dialog.Popup>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

function duplicateLabel(item: ImportPlanResponse["items"][number]) {
  if (item.duplicate.kind === "identical")
    return "Attach source to the existing skill";
  if (item.duplicate.kind === "slugInUse") return "Slug already in use";
  return "New skill";
}
