import { TanStackDevtools } from "@tanstack/react-devtools";
import type { QueryClient } from "@tanstack/react-query";
import { TanStackRouterDevtoolsPanel } from "@tanstack/react-router-devtools";
import { Outlet, createRootRouteWithContext } from "@tanstack/react-router";
import { isFixtureMode } from "@/commands/client";

export const Route = createRootRouteWithContext<{
  queryClient: QueryClient;
}>()({
  component: Root,
});

function Root() {
  return (
    <>
      {isFixtureMode ? (
        <div className="fixture-banner">Fixture mode · no native access</div>
      ) : null}
      <Outlet />
      <TanStackDevtools
        plugins={[
          {
            name: "TanStack Router",
            render: <TanStackRouterDevtoolsPanel />,
          },
        ]}
      />
    </>
  );
}
