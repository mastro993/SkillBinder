import type { DesktopClient } from "@/native/client";

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
    discoveryScan: unstubbed("discoveryScan"),
    importsPrepare: unstubbed("importsPrepare"),
    importsApply: unstubbed("importsApply"),
    libraryList: unstubbed("libraryList"),
    ...overrides,
  };
}
