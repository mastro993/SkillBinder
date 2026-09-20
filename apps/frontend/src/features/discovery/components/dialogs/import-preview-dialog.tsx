import type { ImportPlanResponse } from "@/generated";
import { Button } from "@/components/ui/button";

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
      <section
        className="dialog"
        role="dialog"
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
              </div>
              <span>
                {item.outcome.kind === "attachObservation"
                  ? "Attach source"
                  : "Create skill"}{" "}
                · {item.fileCount} files · {item.totalBytes} bytes
              </span>
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
      </section>
    </div>
  );
}
