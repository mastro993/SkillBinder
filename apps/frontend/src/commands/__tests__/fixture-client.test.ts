import { describe, expect, it } from "vitest";
import { fixtureDesktopClient } from "../fixture-client";

describe("fixture desktop client discovery", () => {
  it("mints a grant, registers it as a mutable root, and removes it", async () => {
    const picked = await fixtureDesktopClient.rootsPick();
    const grant = picked.grant;
    if (!grant) throw new Error("The fixture always mints a grant.");
    expect(grant.resolvedPath).toBe(grant.displayPath);

    const registered = await fixtureDesktopClient.rootsRegister(
      grant.grantId,
      null,
    );
    expect(registered.root.label).toBe("atlas");
    expect(registered.root.enabled).toBe(true);
    expect((await fixtureDesktopClient.rootsList()).roots).toContainEqual(
      registered.root,
    );
    await expect(
      fixtureDesktopClient.rootsRegister(grant.grantId, null),
    ).rejects.toThrow(/grant expired/);

    const updated = await fixtureDesktopClient.rootsUpdate(
      registered.root.rootId,
      "Atlas monorepo",
      false,
    );
    expect(updated.root).toMatchObject({
      label: "Atlas monorepo",
      enabled: false,
    });

    await fixtureDesktopClient.rootsRemove(registered.root.rootId);
    expect((await fixtureDesktopClient.rootsList()).roots).toEqual([]);
  });

  it("reports a running phase, then a finished result with candidates", async () => {
    const { scanId } = await fixtureDesktopClient.discoveryStart();
    const running = await fixtureDesktopClient.discoveryResults(scanId, 0, 50);
    expect(running.phase).toBe("running");
    expect(running.candidates).toEqual([]);
    expect(running.progress.rootsDone).toBeLessThan(
      running.progress.rootsTotal,
    );

    const finished = await fixtureDesktopClient.discoveryResults(scanId, 0, 50);
    expect(finished.phase).toBe("finished");
    expect(finished.limitsReached).toBe(true);
    expect(
      finished.candidates.some(
        (candidate) => candidate.validation.status === "blocked",
      ),
    ).toBe(true);
    expect(finished.candidates.some((candidate) => candidate.linked)).toBe(
      true,
    );
    expect(
      finished.candidates.every(
        (candidate) =>
          candidate.readerAgentLabels.length ===
          candidate.readerAgentIds.length,
      ),
    ).toBe(true);
    expect(finished.totalCandidates).toBe(finished.candidates.length);
    expect(finished.hiddenDuplicates).toBe(1);
    expect(
      finished.candidates.map(({ candidateId }) => candidateId),
    ).not.toContain("fixture-duplicate");
  });

  it("indexes a result page over the candidates the library does not hold", async () => {
    const { scanId } = await fixtureDesktopClient.discoveryStart();
    await fixtureDesktopClient.discoveryResults(scanId, 0, 50);

    const page = await fixtureDesktopClient.discoveryResults(scanId, 3, 2);
    expect(page.offset).toBe(3);
    expect(page.candidates.map(({ candidateId }) => candidateId)).toEqual([
      "fixture-blocked",
      "fixture-root-link",
    ]);
  });

  it("keeps the candidates of a cancelled run", async () => {
    const { scanId } = await fixtureDesktopClient.discoveryStart();
    await fixtureDesktopClient.discoveryCancel(scanId);

    const cancelled = await fixtureDesktopClient.discoveryResults(
      scanId,
      0,
      50,
    );
    expect(cancelled.phase).toBe("cancelled");
    expect(cancelled.candidates.length).toBeGreaterThan(0);
  });
});
