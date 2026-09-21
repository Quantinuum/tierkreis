//! PBS scheduler adapter.

use miette::{Context, IntoDiagnostic, Result, miette};
use serde_json::Value;
use std::{collections::HashMap, path::Path, path::PathBuf};
use tokio::process::Command;

use crate::executor::hpc::spec::{SchedulerStatus, ScriptTemplates};

use super::{JobSpec, SchedulerWrapper};

fn parse_job_id(output: &[u8]) -> Option<String> {
    String::from_utf8_lossy(output)
        .split_whitespace()
        .find(|field| field.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .map(|field| field.split('.').next().unwrap_or(field).to_string())
}

/*
B Array job has at least one subjob running
E Job is exiting after having run
F Job is finished
H Job is held
M Job was moved to another server
Q Job is queued
R Job is running
S Job is suspended
T Job is being moved to new location
U Cycle-harvesting job is suspended due to keyboard activity
W Job is waiting for its submitter-assigned start time to be reached
X Subjob has completed execution or has been deleted
*/

fn parse_job_statuses(output: &[u8]) -> HashMap<String, SchedulerStatus> {
    let Ok(payload) = serde_json::from_slice::<Value>(output) else {
        return HashMap::new();
    };
    let Some(jobs) = payload.get("Jobs").and_then(Value::as_object) else {
        return HashMap::new();
    };
    jobs.iter()
        .filter_map(|(job_id, job)| {
            let job_id = job_id.split('.').next().unwrap_or(job_id).to_string();
            let state = job.get("job_state")?.as_str()?;
            let exit_status = job.get("Exit_status").and_then(Value::as_i64);
            let status = match state {
                "Q" | "H" | "W" | "T" => SchedulerStatus::Queued,
                "B" | "E" | "R" | "S" | "U" => SchedulerStatus::Running,
                "F" | "X" if exit_status == Some(0) => SchedulerStatus::Complete,
                "F" | "X" => SchedulerStatus::Error {
                    message: format!(
                        "PBS job {job_id} failed: state={state}, exit_status={}",
                        exit_status.map_or_else(|| "unknown".to_string(), |code| code.to_string())
                    ),
                },
                _ => SchedulerStatus::Error {
                    message: format!("PBS job {job_id} failed: state={state}"),
                },
            };
            Some((job_id, status))
        })
        .collect()
}

/// PBS scheduler using `qsub`, `qstat`, and `qdel`.
#[derive(Clone, Debug)]
pub struct PbsWrapper {
    /// Submission command.
    pub qsub: PathBuf,
    /// Status command.
    pub qstat: PathBuf,
    /// Cancellation command.
    pub qdel: PathBuf,
    templates: ScriptTemplates,
}

impl Default for PbsWrapper {
    fn default() -> Self {
        Self {
            qsub: "qsub".into(),
            qstat: "qstat".into(),
            qdel: "qdel".into(),
            templates: ScriptTemplates::default(),
        }
    }
}

impl PbsWrapper {
    /// Construct a wrapper for the repository's local PBS Docker shims.
    #[must_use]
    #[cfg(test)]
    pub fn local() -> Self {
        let binaries = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../infra/pbs_local");
        Self {
            qsub: binaries.join("qsub"),
            qstat: binaries.join("qstat"),
            qdel: binaries.join("qdel"),
            ..Self::default()
        }
    }

    /// Construct a PBS wrapper using shared submission templates.
    #[must_use]
    pub fn with_templates(templates: ScriptTemplates) -> Self {
        Self {
            templates,
            ..Self::default()
        }
    }
}

impl SchedulerWrapper for PbsWrapper {
    async fn submit(&self, spec: JobSpec, script_path: &Path) -> Result<String> {
        let scheduler = self.clone();
        let script_path = script_path.to_path_buf();
        std::fs::write(&script_path, scheduler.templates.render("pbs", &spec)?)
            .into_diagnostic()?;
        let output = Command::new(&scheduler.qsub)
            .arg(&script_path)
            .output()
            .await
            .into_diagnostic()
            .wrap_err("Failed to invoke qsub")?;
        if !output.status.success() {
            return Err(miette!(
                "qsub failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        parse_job_id(&output.stdout).ok_or_else(|| miette!("qsub returned no job id"))
    }

    async fn check(&self, job_ids: Vec<String>) -> Result<HashMap<String, SchedulerStatus>> {
        let scheduler = self.clone();
        if job_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let output = Command::new(&scheduler.qstat)
            .args([
                "-x", // Include finished/moved jobs
                "-f", // Long format
                "-F", "json", // Output in JSON format
            ])
            .args(&job_ids)
            .output()
            .await
            .into_diagnostic()
            .wrap_err("Failed to invoke qstat")?;
        if !output.status.success() {
            return Err(miette!(
                "qstat failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(parse_job_statuses(&output.stdout))
    }

    async fn cancel(&self, job_id: String) -> Result<()> {
        let scheduler = self.clone();
        let status = Command::new(&scheduler.qdel)
            .arg(job_id)
            .status()
            .await
            .into_diagnostic()?;
        status
            .success()
            .then_some(())
            .ok_or_else(|| miette!("qdel failed: {status}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pbs_job_id() {
        assert_eq!(parse_job_id(b"1234.server\n"), Some("1234".to_string()));
    }

    #[test]
    fn parses_pbs_job_statuses() {
        let statuses = parse_job_statuses(
            br#"{
  "Jobs": {
    "1.server": { "job_state": "Q" },
    "2.server": { "job_state": "R" },
    "3.server": { "job_state": "F", "Exit_status": 0 },
    "4.server": { "job_state": "F", "Exit_status": 137 }
  }
}"#,
        );

        assert_eq!(statuses.get("1"), Some(&SchedulerStatus::Queued));
        assert_eq!(statuses.get("2"), Some(&SchedulerStatus::Running));
        assert_eq!(statuses.get("3"), Some(&SchedulerStatus::Complete));
        assert_eq!(
            statuses.get("4"),
            Some(&SchedulerStatus::Error {
                message: "PBS job 4 failed: state=F, exit_status=137".to_string(),
            })
        );
    }
}
