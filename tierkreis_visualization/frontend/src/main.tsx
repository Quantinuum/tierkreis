import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import React, { Suspense } from "react";
import ReactDOM from "react-dom/client";
import { routeTree } from "./routeTree.gen";
import { ReactFlowProvider } from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import "./index.css";
import { createRouter, RouterProvider } from "@tanstack/react-router";

const queryClient = new QueryClient();
const router = createRouter({ routeTree });
declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <QueryClientProvider client={queryClient}>
    <React.StrictMode>
      <Suspense fallback="Loading...">
        <ReactFlowProvider>
          <RouterProvider router={router} />
        </ReactFlowProvider>
      </Suspense>
    </React.StrictMode>
  </QueryClientProvider>
);
