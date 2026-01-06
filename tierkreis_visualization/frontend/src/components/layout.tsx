import { NodeInfo } from "@/components/info";
import { Dialog } from "@/components/ui/dialog";
import { SidebarProvider } from "@/components/ui/sidebar";
import { WorkflowSidebar } from "@/components/workflow-sidebar";
import { InfoProps } from "@/components/types";
import { WorkflowDisplay } from "@/data/api_types";
import { Breadcrumbs } from "./breadcrumbs";

export default function Layout({
  children,
  workflows,
  workflowId,
  info,
  loc,
}: {
  children: React.ReactNode;
  workflows: WorkflowDisplay[];
  workflowId: string;
  info: InfoProps;
  loc: string;
}) {
  return (
    <SidebarProvider>
      <Dialog>
        <WorkflowSidebar workflows={workflows} workflowId={workflowId} />
        <NodeInfo info={info} />
        <main className="flex flex-col">
          <Breadcrumbs wid={workflowId} loc={loc} />
          {children}
        </main>
      </Dialog>
    </SidebarProvider>
  );
}
