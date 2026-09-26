import type { DesktopClient } from "@/commands/client";

function unstubbed(method: keyof DesktopClient) {
  return () => {
    throw new Error(`DesktopClient.${method} is not stubbed for this test.`);
  };
}

/** Builds a client whose unstubbed methods fail loudly instead of returning undefined. */
export function stubDesktopClient(
  overrides: Partial<DesktopClient>,
): DesktopClient {
  return {
    bootstrap: unstubbed("bootstrap"),
    updateOnboardingProgress: unstubbed("updateOnboardingProgress"),
    completeLocalOnboarding: unstubbed("completeLocalOnboarding"),
    rootsPick: unstubbed("rootsPick"),
    rootsRegister: unstubbed("rootsRegister"),
    rootsList: unstubbed("rootsList"),
    rootsUpdate: unstubbed("rootsUpdate"),
    rootsRemove: unstubbed("rootsRemove"),
    discoveryStart: unstubbed("discoveryStart"),
    discoveryResults: unstubbed("discoveryResults"),
    discoveryCancel: unstubbed("discoveryCancel"),
    discoveryCurrent: unstubbed("discoveryCurrent"),
    importsPrepare: unstubbed("importsPrepare"),
    importsApply: unstubbed("importsApply"),
    libraryList: unstubbed("libraryList"),
    diagnosticsRevealLogs: unstubbed("diagnosticsRevealLogs"),
    gitSyncStatus: unstubbed("gitSyncStatus"),
    gitSyncConnect: unstubbed("gitSyncConnect"),
    gitSyncRefresh: unstubbed("gitSyncRefresh"),
    gitSyncPull: unstubbed("gitSyncPull"),
    gitSyncPush: unstubbed("gitSyncPush"),
    gitSync: unstubbed("gitSync"),
    gitSyncDisconnect: unstubbed("gitSyncDisconnect"),
    ...overrides,
  };
}
