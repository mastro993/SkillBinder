import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { BookOpen, Plus } from "lucide-react";
import { bootstrapQuery } from "@/app/bootstrap-query";
import { Button } from "@/components/ui/button";
import { libraryListQuery } from "../queries";

export function LibraryView() {
  const bootstrap = useQuery(bootstrapQuery);
  const library = useQuery(libraryListQuery);
  if (bootstrap.isPending || library.isPending)
    return <p className="page-status">Loading library…</p>;
  if (bootstrap.isError || library.isError)
    return <p className="page-status error">Library state unavailable.</p>;
  if (!bootstrap.data.onboarding.completed) {
    return (
      <div className="empty-panel">
        <h1>Setup is not finished</h1>
        <Link to="/onboarding">Resume setup</Link>
      </div>
    );
  }
  const data = library.data;
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
      {data.hasUncommittedChanges ? (
        <p className="library-dirty-line">
          Imported content is not committed yet.
        </p>
      ) : null}
      {data.skills.length === 0 ? (
        <div className="empty-panel">
          <span className="empty-icon">
            <BookOpen size={26} />
          </span>
          <h2>Your library is ready</h2>
          <p>
            No skills imported yet. Visit Discovery to inspect local skill
            folders.
          </p>
        </div>
      ) : (
        <div className="library-grid">
          {data.skills.map((skill) => (
            <article className="library-card" key={skill.skillId}>
              <div className="library-card-header">
                <div>
                  <h2>{skill.displayName ?? skill.slug}</h2>
                  <p className="table-secondary">{skill.slug}</p>
                </div>
                <span className={`status-pill ${skill.validation.status}`}>
                  {skill.validation.status}
                </span>
              </div>
              <p className="library-description">
                {skill.description ?? "No description"}
              </p>
              <p className="library-meta">
                {skill.fileCount} files · {skill.totalBytes} bytes
              </p>
              {skill.validation.messages.map((message) => (
                <p className="table-secondary" key={message.code}>
                  {message.message}
                </p>
              ))}
              <ul className="source-list">
                {skill.sources.map((source) => (
                  <li key={source.displayPath}>
                    <span>{source.displayPath}</span>
                    <small>
                      {source.readerAgentIds.join(", ") || "Unknown reader"}
                    </small>
                  </li>
                ))}
              </ul>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}
