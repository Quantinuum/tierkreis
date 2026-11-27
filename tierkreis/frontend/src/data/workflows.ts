import { AttemptDisplay, WorkflowDisplay } from "./api_types";

export function attemptStatus(attempt: AttemptDisplay) {
  if (attempt.error_time) return { label: "Error", className: "text-nexus-red" };
  if (attempt.cancelled_time) return { label: "Cancelled", className: "text-muted-foreground" };
  if (attempt.complete_time) return { label: "Success", className: "text-nexus-green" };
  if (attempt.start_time) return { label: "Running", className: "text-amber-600" };
  if (attempt.queued_time) return { label: "Queued", className: "text-amber-600" };
  return { label: "Not started", className: "text-muted-foreground" };
}

export function latestAttempt(workflow: WorkflowDisplay) {
  return workflow.runs.flatMap((run) => run.attempts).reduce<AttemptDisplay | undefined>(
    (latest, attempt) => {
      const time = attempt.start_time ?? attempt.queued_time ?? "";
      const latestTime = latest?.start_time ?? latest?.queued_time ?? "";
      return !latest || time > latestTime ? attempt : latest;
    },
    undefined,
  );
}
