import { NodeInfo } from "@/components/info";
import { Dialog } from "@/components/ui/dialog";
import { SidebarProvider } from "@/components/ui/sidebar";
import { WorkflowSidebar } from "@/components/workflow-sidebar";
import { InfoProps } from "@/components/types";
import { WorkflowDisplay } from "@/data/api_types";
import { Breadcrumbs } from "./breadcrumbs";
import { useState } from "react";

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
  const [open, setOpen] = useState(false);
  const dialogCloser = () => {
    setOpen(false);
  };
  return (
    <SidebarProvider>
      <Dialog open={open} onOpenChange={setOpen}>
        <WorkflowSidebar workflows={workflows} workflowId={workflowId} />
        <NodeInfo info={info} closer={dialogCloser} />
        <main className="flex flex-col">
          <Breadcrumbs wid={workflowId} loc={loc} />
          {children}
        </main>
      </Dialog>
    </SidebarProvider>
  );
}
