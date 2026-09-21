---
name: tanstack-router
description: "Add or change a route in the SkillBinder frontend (apps/frontend). Use when adding, moving, or renaming a file under src/routes, wiring a feature screen to a URL, editing the router instance in src/router.tsx, regenerating src/routeTree.gen.ts, or debugging a stale route tree, an unmatched path, a redirect, or a route-level data load. Triggers on src/routes, createFileRoute, createRootRouteWithContext, routeTree.gen.ts, generate-routes, tsr.config.json, createHashHistory, beforeLoad, redirect, RouterProvider."
---

# TanStack Router in SkillBinder

File-based routing, SPA, hash history. Verified against `@tanstack/react-router` 1.170.38
(`@tanstack/router-core` 1.171.32), `@tanstack/router-plugin` 1.168.40,
`@tanstack/router-cli` 1.167.38, and `@tanstack/react-query` 5.103.1 as installed in
`apps/frontend`.

## Where things live

| Path | Role |
| --- | --- |
| `apps/frontend/src/routes/*.tsx` | Route files. One file per route. |
| `apps/frontend/src/routes/__root.tsx` | Root route. Declares the router context type and mounts the devtools panel. |
| `apps/frontend/src/router.tsx` | Router instance, hash history, `Register` augmentation. |
| `apps/frontend/src/routeTree.gen.ts` | Generated. Never edit. |
| `apps/frontend/tsr.config.json` | Generator input and output directories. |
| `apps/frontend/src/features/<feature>/screens/<view>.tsx` | Screens that route files import. |

## Add a route

1. Create the screen at `apps/frontend/src/features/<feature>/screens/<view>.tsx` if it does
   not exist.
2. Create the route file.
3. Run `pnpm --filter @skillbinder/frontend generate-routes`.
4. Run `pnpm --filter @skillbinder/frontend typecheck`.

A route inside the app chrome:

```tsx
import { createFileRoute } from "@tanstack/react-router";
import { ReportsView } from "@/features/reports/screens/reports-view";

export const Route = createFileRoute("/_shell/reports")({
  component: ReportsView,
});
```

A route outside it:

```tsx
import { createFileRoute } from "@tanstack/react-router";
import { OnboardingView } from "@/features/onboarding/screens/onboarding-view";

export const Route = createFileRoute("/onboarding")({
  component: OnboardingView,
});
```

Rules:

- Export the route as `Route`. The generator and the code splitter both look for that name.
- Import the screen from its concrete file. This repo has no barrel files, so
  `@/features/reports` does not resolve; use `@/features/reports/screens/reports-view`.
- The path string in `createFileRoute(...)` must match the generated route id.
  `tsr generate` maintains it, and a mismatch fails typecheck.

## Filenames and the `_shell.` prefix

- `_shell.tsx` is a pathless layout route. It renders `AppShell` around an `<Outlet />` and
  adds no URL segment, so `createFileRoute("/_shell")`.
- `_shell.<name>.tsx` is a child of it: `_shell.discovery.tsx` has id `/_shell/discovery` and
  full path `/discovery`.
- `<name>.tsx` at the routes root is top level: `onboarding.tsx` has path `/onboarding`.
- `index.tsx` is `/`.

Add a `_shell.` file only when the route must render inside the app chrome. Onboarding sits
outside it on purpose.

## Router instance

`src/router.tsx` owns the single router and the type registration:

```tsx
export const router = createRouter({
  routeTree,
  context: { queryClient },
  history: createHashHistory(),
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
```

- Keep `createRouter` and the `declare module` block in this file. `Register` is what makes
  `Link to="/discovery"`, `redirect({ to: ... })`, and `navigate` check against real paths.
- `context: { queryClient }` satisfies `createRootRouteWithContext<{ queryClient: QueryClient }>()`
  in `__root.tsx`. Extend that context type there, not in individual route files.
- `src/main.tsx` only mounts: `QueryClientProvider` wrapping `RouterProvider router={router}`.

## Hash history

`createHashHistory()` keeps the whole route in the URL fragment. Tauri serves the built
bundle from its own protocol with no server-side rewrite, so path-based history would break
reload and deep links. Do not switch to `createBrowserHistory` without changing how the shell
serves `dist/`.

## Data loading

Route-level data goes through the shared QueryClient, not a bare call in the component.

`src/routes/index.tsx` is the reference: it loads before rendering and redirects.

```tsx
export const Route = createFileRoute("/")({
  beforeLoad: async ({ context }) => {
    const bootstrap = await context.queryClient.ensureQueryData(bootstrapQuery);
    throw redirect({
      to: bootstrap.onboarding.completed ? "/discovery" : "/onboarding",
    });
  },
});
```

- `context.queryClient` is the router context. `ensureQueryData` shares cache with any
  component reading the same key.
- `beforeLoad` must `throw redirect(...)`, never return it.
- Query definitions live with the feature (`features/<feature>/hooks/queries.ts`); shared
  ones in `src/lib/`. Screens read them with `useQuery`.
- `Route.useSearch()` and `Route.useParams()` give typed params. Declare a search shape with
  `validateSearch` on the route; do not parse `location.search` by hand.

## Code splitting, generation, and devtools

`vite.config.ts` registers the plugin:

```ts
tanstackRouter({ target: "react", autoCodeSplitting: true }),
```

- `autoCodeSplitting: true` moves `component`, `loader`, and the other route options of each
  route file into its own chunk. Nothing to do per route, and do not hand-write `.lazy.tsx`
  files.
- Export only `Route` from a route file. Any other export is skipped by the splitter and
  lands in the entry chunk; the plugin warns
  `[tanstack-router] These exports from "<file>" will not be code-split`.
- The vite plugin regenerates `routeTree.gen.ts` on dev and build unless
  `enableRouteGeneration: false` is set.
- `__root.tsx` mounts the panel:
  `<TanStackDevtools plugins={[{ name, render: <TanStackRouterDevtoolsPanel /> }]} />`.
  `@tanstack/devtools-vite` 0.8.5 (`devtools()` in `vite.config.ts`) deletes that element and
  the now-unused panel import on `vite build`; `removeDevtoolsOnBuild` defaults to true. Do not
  add build-time conditionals or `import.meta.env.DEV` guards for devtools.
- The stripper only acts on files importing a package in its list (`@tanstack/react-devtools`
  and the other framework devtools packages) and only removes `render` values it recognizes:
  a JSX element, an identifier, or a zero-arg function returning one. A panel wired up any
  other way stays in the production bundle.

## Generated route tree

`src/routeTree.gen.ts` is written by the generator. It carries `// @ts-nocheck` and is
excluded from the formatter. Never edit or hand-fix it. Regenerate it:

```bash
pnpm --filter @skillbinder/frontend generate-routes
```

That script runs `tsr generate` (`@tanstack/router-cli`). `tsr.config.json` pins the
directories it reads and writes, resolved relative to `apps/frontend`:

```json
{
  "routesDirectory": "./src/routes",
  "generatedRouteTree": "./src/routeTree.gen.ts"
}
```

Both values are the generator defaults; the file exists so the paths are explicit.

## Verify

```bash
pnpm --filter @skillbinder/frontend generate-routes
pnpm --filter @skillbinder/frontend typecheck
pnpm --filter @skillbinder/frontend test
pnpm --filter @skillbinder/frontend build
pnpm --filter @skillbinder/frontend dev          # vite on 127.0.0.1:1420, the Tauri devUrl
pnpm --filter @skillbinder/frontend dev:fixture  # same port, fixture IPC client, no Tauri
```

`typecheck` is what catches a route id that is missing from the tree: `createFileRoute("/x")`
only accepts keys of `FileRoutesByPath`, which the generated file provides.

`dev:fixture` is the one to use for browser checks. It runs `--mode fixture`, which selects
`src/commands/fixture-client.ts` instead of the Tauri IPC client, so a routing change can be
exercised end to end without the desktop shell. Plain `dev` is for the Tauri devUrl and its
IPC calls fail in a browser. The root script `pnpm dev:frontend` is the fixture server.

## Failure modes

| Symptom | Cause | Fix |
| --- | --- | --- |
| `Argument of type '"/reports"' is not assignable` after adding a file | `routeTree.gen.ts` is stale, `FileRoutesByPath` has no key for the new file | `pnpm --filter @skillbinder/frontend generate-routes` |
| `Cannot find module '@/features/reports'` | Imported a feature barrel; this repo has no barrel files | Import `@/features/reports/screens/reports-view` |
| New path renders the default `<p>Not Found</p>` | The tree was generated before the file was added | Regenerate, then reload |
| `Link to="/x"` typechecks but the page 404s | `to` was written by hand against an old path | Regenerate and let the type error point at the real path |
| A route chunk is missing from the build output | Extra exports in the route file, skipped by the splitter | Export only `Route` |
| The devtools overlay shows up in a production build | `removeDevtoolsOnBuild: false` was set, or the panel was mounted in a form the stripper does not recognize | Remove the override; mount it as `<TanStackDevtools plugins={[{ render: <Panel /> }]} />` |
| Route change has no effect in the running app | Dev server is serving a tree generated before the edit | Regenerate; restart `dev` if it still lags |
