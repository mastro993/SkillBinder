import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { BookOpen, FolderSearch, Plus } from "lucide-react";
import { bootstrapQuery } from "@/app/bootstrap-query";
import { Button } from "@/components/ui/button";

export function LibraryView() {
  const bootstrap = useQuery(bootstrapQuery);
  if (bootstrap.isPending)
    return <p className="page-status">Loading library…</p>;
  if (bootstrap.isError)
    return <p className="page-status error">Library state unavailable.</p>;
  if (!bootstrap.data.onboarding.completed) {
    return (
      <div className="empty-panel">
        <h1>Setup is not finished</h1>
        <Link to="/onboarding">Resume setup</Link>
      </div>
    );
  }
  return (
    <section className="page">
      <header className="page-header">
        <div>
          <p className="eyebrow">Canonical collection</p>
          <h1>Library</h1>
        </div>
        <Button disabled>
          <Plus size={16} /> New skill
        </Button>
      </header>
      <div className="empty-panel">
        <span className="empty-icon">
          <BookOpen size={26} />
        </span>
        <h2>Your library is ready</h2>
        <p>
          Discovery and skill creation arrive in their dedicated milestones.
          This local Git library is ready for them.
        </p>
        <div className="empty-detail">
          <FolderSearch size={17} /> No skills imported
        </div>
      </div>
    </section>
  );
}
