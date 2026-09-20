import type { QueryClient } from "@tanstack/react-query";
import { Outlet, createRootRouteWithContext } from "@tanstack/react-router";
import { isFixtureMode } from "@/native/client";

type RouterContext = { queryClient: QueryClient };

export const Route = createRootRouteWithContext<RouterContext>()({
  component: Root,
});

function Root() {
  return (
    <>
      {isFixtureMode ? (
        <div className="fixture-banner">Fixture mode · no native access</div>
      ) : null}
      <Outlet />
    </>
  );
}
