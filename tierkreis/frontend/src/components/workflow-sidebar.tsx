import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarSeparator,
} from "@/components/ui/sidebar";
import { Link, useSearch } from "@tanstack/react-router";
import { WorkflowDisplay } from "@/data/api_types";
import { attemptStatus } from "@/data/workflows";
import { Fragment } from "react";

export function WorkflowSidebar({
  workflows,
  workflowId,
}: {
  workflows: WorkflowDisplay[];
  workflowId: string;
}) {
  const { attempt } = useSearch({ from: "/runs/_/$wrid/nodes/$loc/" });
  return (
    <Sidebar>
      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupLabel>
            <Link to="/">Workflows</Link>
          </SidebarGroupLabel>
        </SidebarGroup>
        {workflows.map((workflow, index) => (
          <Fragment key={workflow.workflow_id}>
            {index > 0 && <SidebarSeparator />}
            <SidebarGroup>
              <SidebarGroupLabel title={workflow.workflow_id}>
                <span className="truncate">
                  {workflow.name ?? "Unnamed Workflow"}
                </span>
              </SidebarGroupLabel>
              <SidebarGroupContent>
                {workflow.runs.length === 0 && (
                  <p className="px-2 text-xs text-muted-foreground">
                    No runs yet.
                  </p>
                )}
                {workflow.runs.map((run) => (
                  <div key={run.workflow_run_id} className="mb-2">
                    <p className="px-2 py-1 text-xs break-all">
                      Run {run.workflow_run_id}
                    </p>
                    <SidebarMenu>
                      {run.attempts.map((item) => {
                        const status = attemptStatus(item);
                        return (
                          <SidebarMenuItem key={item.attempt}>
                            <SidebarMenuButton
                              asChild
                              isActive={
                                workflowId === run.workflow_run_id &&
                                attempt === item.attempt
                              }
                            >
                              <Link
                                to="/runs/$wrid/nodes/$loc"
                                params={{ wrid: run.workflow_run_id, loc: "-" }}
                                search={{ attempt: item.attempt }}
                              >
                                <span>Attempt {item.attempt}</span>
                                <span className={`ml-auto ${status.className}`}>
                                  {status.label}
                                </span>
                              </Link>
                            </SidebarMenuButton>
                          </SidebarMenuItem>
                        );
                      })}
                    </SidebarMenu>
                  </div>
                ))}
              </SidebarGroupContent>
            </SidebarGroup>
          </Fragment>
        ))}
      </SidebarContent>
    </Sidebar>
  );
}
