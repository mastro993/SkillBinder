import { Cancel01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import type { ImportPlanResponse } from "@/types";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

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
    <Dialog
      open
      modal
      onOpenChange={(open: boolean) => {
        if (!open && !applying) onClose();
      }}
    >
      <DialogContent className="max-h-[85vh] overflow-y-auto sm:max-w-2xl">
        <DialogClose
          render={
            <Button
              variant="ghost"
              className="absolute top-4 right-4"
              size="icon-sm"
              aria-label="Close import review"
              disabled={applying}
            />
          }
        >
          <HugeiconsIcon icon={Cancel01Icon} aria-hidden="true" />
        </DialogClose>
        <DialogHeader>
          <p className="text-xs font-extrabold tracking-widest text-success uppercase">
            Import review
          </p>
          <DialogTitle>Review import plan</DialogTitle>
        </DialogHeader>
        <DialogDescription>
          SkillBinder will copy these sources into your local library. Originals
          stay unchanged.
        </DialogDescription>
        {plan.items.some((item) => item.outcome.kind === "conflict") ? (
          <Alert variant="warning" role="note">
            <AlertDescription>
              A slug in this plan is already used by another skill, so the
              import adds a second copy. Choose which copy to keep in the
              Library.
            </AlertDescription>
          </Alert>
        ) : null}
        <ul className="grid gap-2">
          {plan.items.map((item) => (
            <li
              key={item.candidateId}
              className="grid gap-1 rounded-lg border px-3 py-2.5 text-sm"
            >
              <div className="flex items-center justify-between gap-3">
                <span className="font-semibold">{item.slug}</span>
                <span className="text-xs text-muted-foreground">
                  Validation: {validationLabels[item.validation.status]}
                </span>
              </div>
              <span className="break-words text-xs text-muted-foreground">
                {item.displayPath}
              </span>
              <span className="text-xs text-muted-foreground">
                Destination skill: <code>{item.skillId}</code>
              </span>
              <span className="text-xs text-muted-foreground">
                {duplicateLabel(item)} · {item.fileCount} files ·{" "}
                {item.totalBytes} bytes
              </span>
              {item.validation.messages.map((message) => (
                <span
                  className="text-xs text-muted-foreground"
                  key={message.code}
                >
                  {message.message}
                </span>
              ))}
              {item.exclusions.length ? (
                <span className="text-xs text-warning">
                  Warnings: {item.exclusions.join("; ")}
                </span>
              ) : null}
            </li>
          ))}
        </ul>
        <DialogFooter>
          <Button variant="ghost" onClick={onClose} disabled={applying}>
            Back
          </Button>
          <Button onClick={onApply} disabled={applying}>
            {applying ? "Importing…" : "Apply import"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function duplicateLabel(item: ImportPlanResponse["items"][number]) {
  if (item.outcome.kind === "conflict") {
    const owners = item.outcome.skillIds.length;
    return `Adds a second copy: ${owners === 1 ? "another skill already uses" : `${owners} skills already use`} this slug`;
  }
  if (item.duplicate.kind === "identical")
    return "Attach source to the existing skill";
  if (item.duplicate.kind === "slugInUse") return "Slug already in use";
  return "New skill";
}
