import { useQuery } from "@tanstack/react-query";
import { bootstrapQuery } from "@/app/bootstrap-query";

export function SettingsView() {
  const bootstrap = useQuery(bootstrapQuery);
  return (
    <section className="page">
      <header className="page-header">
        <div>
          <p className="eyebrow">This device</p>
          <h1>Settings</h1>
        </div>
      </header>
      <div className="settings-list">
        <Setting
          label="Application"
          value={
            bootstrap.data
              ? `SkillBinder ${bootstrap.data.appVersion}`
              : "Loading…"
          }
        />
        <Setting
          label="Git"
          value={
            bootstrap.data?.git.version
              ? `Ready · ${bootstrap.data.git.version}`
              : "Needs attention"
          }
        />
        <Setting
          label="Library"
          value={
            bootstrap.data?.libraryState === "ready"
              ? "Local library ready"
              : "Not created"
          }
        />
        <Setting label="Remote sync" value="Not connected · optional" />
      </div>
    </section>
  );
}

function Setting({ label, value }: { label: string; value: string }) {
  return (
    <div className="setting-row">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}
