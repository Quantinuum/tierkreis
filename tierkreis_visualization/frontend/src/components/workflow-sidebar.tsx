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
import { Workflow } from "@/components/types";

export function WorkflowSidebar({
  workflows,
  workflowId,
}: {
  workflows: Workflow[];
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
              {workflows.map((workflow) => (
                <SidebarMenuItem key={workflow.id}>
                  <SidebarMenuButton
                    asChild
                    isActive={workflowId === workflow.id}
                  >
                    <Link
                      to={"/workflows/$wid/nodes/$loc"}
                      params={{ wid: workflow.id, loc: "-" }}
                    >
                      <span>{workflow.name}</span>
                    </Link>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
    </Sidebar>
  );
}
