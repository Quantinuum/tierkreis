import { createFileRoute } from "@tanstack/react-router";
import { BuilderPage } from "./-components/BuilderPage";

export const Route = createFileRoute("/builder/")({
  component: RouteComponent,
});

function RouteComponent() {
  return (
    <div className="h-screen w-screen">
      <BuilderPage />
    </div>
  );
}
