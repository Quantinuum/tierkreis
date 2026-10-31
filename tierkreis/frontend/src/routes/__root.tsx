import { createRootRoute, HeadContent, Outlet } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";

const RootLayout = () => (
  <>
    <HeadContent />
    <Outlet />
    <TanStackRouterDevtools />
  </>
);

export const Route = createRootRoute({
  component: RootLayout,
});
