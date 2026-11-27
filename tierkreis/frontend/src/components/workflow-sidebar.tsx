import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar";
import { Link } from "@tanstack/react-router";
import { WorkflowDisplay } from "@/data/api_types";

export function WorkflowSidebar({
  workflows,
  workflowId,
}: {
  workflows: WorkflowDisplay[];
  workflowId: string;
}) {
  return (
    <Sidebar>
      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupLabel>
            <Link to="/"> Workflows</Link>
          </SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {workflows.map((workflow) => {
                const wrid = workflow.runs[0]?.workflow_run_id ?? "-";
                return (
                  <SidebarMenuItem key={wrid}>
                    <SidebarMenuButton asChild isActive={workflowId === wrid}>
                      <Link
                        to={"/runs/$wrid/nodes/$loc"}
                        params={{ wrid: wrid, loc: "-" }}
                      >
                        <span>{workflow.name}</span>
                      </Link>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                );
              })}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
    </Sidebar>
  );
}
