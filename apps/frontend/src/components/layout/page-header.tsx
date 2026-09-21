import type { ReactNode } from "react";

export function PageHeader({
  eyebrow,
  title,
  lead,
  action,
}: {
  eyebrow: string;
  title: string;
  lead?: string;
  action?: ReactNode;
}) {
  return (
    <header className="mb-9 flex items-center justify-between gap-4">
      <div>
        <p className="mb-2 text-xs font-extrabold tracking-widest text-success uppercase">
          {eyebrow}
        </p>
        <h1 className="text-title">{title}</h1>
        {lead ? (
          <p className="mt-2.5 max-w-[610px] text-muted-foreground">{lead}</p>
        ) : null}
      </div>
      {action}
    </header>
  );
}
