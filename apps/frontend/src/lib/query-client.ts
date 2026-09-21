import { QueryClient } from "@tanstack/react-query";

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      networkMode: "always",
      retry: false,
      staleTime: 15_000,
      refetchOnWindowFocus: true,
    },
    mutations: { networkMode: "always", retry: false },
  },
});
