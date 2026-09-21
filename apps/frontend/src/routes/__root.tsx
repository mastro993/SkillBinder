import { TanStackDevtools } from "@tanstack/react-devtools";
import type { QueryClient } from "@tanstack/react-query";
import { TanStackRouterDevtoolsPanel } from "@tanstack/react-router-devtools";
import { Outlet, createRootRouteWithContext } from "@tanstack/react-router";
import { Badge } from "@/components/ui/badge";
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
        <Badge
          variant="warning"
          className="fixed top-2 left-1/2 z-20 -translate-x-1/2"
        >
          Fixture mode · no native access
        </Badge>
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
