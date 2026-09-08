//! Slurm scheduler adapter.

use futures::FutureExt;
use miette::{Context, IntoDiagnostic, Result, miette};
use std::{collections::HashMap, path::Path, path::PathBuf};
use tokio::process::Command;

use crate::executor::hpc::spec::{SchedulerStatus, ScriptTemplates};

use super::{JobSpec, SchedulerWrapper};

fn parse_job_statuses(output: &[u8]) -> HashMap<String, SchedulerStatus> {
    String::from_utf8_lossy(output)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let mut fields = line.split('|');
            let job_id = fields.next()?.trim();
            let state = fields.next()?.split(['+', ' ']).next().unwrap_or_default();
            let code = fields
                .next()
                .unwrap_or_default()
                .split(':')
                .next()
                .unwrap_or("1");
            let status = match state {
                "PENDING" | "CONFIGURING" | "REQUEUED" | "REQUEUE_FED" | "REQUEUE_HOLD"
                | "RESIZING" => SchedulerStatus::Queued,
                "RUNNING" | "COMPLETING" | "STAGE_OUT" | "SUSPENDED" => SchedulerStatus::Running,
                "COMPLETED" if code == "0" => SchedulerStatus::Complete,
                "CANCELLED" => SchedulerStatus::Cancelled,
                _ => SchedulerStatus::Error {
                    message: format!("Slurm job {job_id} failed: state={state}, exit_code={code}"),
                },
            };
            Some((job_id.to_string(), status))
        })
        .collect()
}

/// Slurm scheduler using `sbatch`, `sacct`, and `scancel`.
#[derive(Clone, Debug)]
pub struct SlurmWrapper {
    /// Submission command.
    pub sbatch: PathBuf,
    /// Accounting command.
    pub sacct: PathBuf,
    /// Cancellation command.
    pub scancel: PathBuf,
    templates: ScriptTemplates,
}

impl Default for SlurmWrapper {
    fn default() -> Self {
        Self {
            sbatch: "sbatch".into(),
            sacct: "sacct".into(),
            scancel: "scancel".into(),
            templates: ScriptTemplates::default(),
        }
    }
}

impl SlurmWrapper {
    /// Construct a wrapper for the repository's local Slurm Docker shims.
    #[must_use]
    #[cfg(test)]
    pub fn local() -> Self {
        let binaries = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../infra/slurm_local");
        Self {
            sbatch: binaries.join("sbatch"),
            sacct: binaries.join("sacct"),
            scancel: binaries.join("scancel"),
            ..Self::default()
        }
    }

    /// Construct a Slurm wrapper using shared submission templates.
    #[must_use]
    pub fn with_templates(templates: ScriptTemplates) -> Self {
        Self {
            templates,
            ..Self::default()
        }
    }
}

impl SchedulerWrapper for SlurmWrapper {
    fn submit(
        &self,
        spec: JobSpec,
        script_path: &Path,
    ) -> futures::future::BoxFuture<'_, Result<String>> {
        let scheduler = self.clone();
        let script_path = script_path.to_path_buf();
        async move {
            std::fs::write(&script_path, scheduler.templates.render("slurm", &spec)?)
                .into_diagnostic()?;
            let output = Command::new(&scheduler.sbatch)
                .args(["--parsable"])
                .arg(&script_path)
                .output()
                .await
                .into_diagnostic()
                .wrap_err("Failed to invoke sbatch")?;
            if !output.status.success() {
                return Err(miette!(
                    "sbatch failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .split(';')
                .next()
                .filter(|id| !id.is_empty())
                .map(str::to_string)
                .ok_or_else(|| miette!("sbatch returned no job id"))
        }
        .boxed()
    }

    fn check(
        &self,
        job_ids: Vec<String>,
    ) -> futures::future::BoxFuture<'_, Result<HashMap<String, SchedulerStatus>>> {
        let scheduler = self.clone();
        async move {
            if job_ids.is_empty() {
                return Ok(HashMap::new());
            }
            let job_ids = job_ids.join(",");
            let output = Command::new(&scheduler.sacct)
                .args([
                    "-X",
                    "-n",
                    "-P",
                    "-o",
                    "JobIDRaw,State,ExitCode",
                    "-j",
                    &job_ids,
                ])
                .output()
                .await
                .into_diagnostic()
                .wrap_err("Failed to invoke sacct")?;
            if !output.status.success() {
                return Err(miette!(
                    "sacct failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            Ok(parse_job_statuses(&output.stdout))
        }
        .boxed()
    }

    fn cancel(&self, job_id: String) -> futures::future::BoxFuture<'_, Result<()>> {
        let scheduler = self.clone();
        async move {
            let status = Command::new(&scheduler.scancel)
                .arg(job_id)
                .status()
                .await
                .into_diagnostic()?;
            status
                .success()
                .then_some(())
                .ok_or_else(|| miette!("scancel failed: {status}"))
        }
        .boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_slurm_job_statuses() {
        let statuses = parse_job_statuses(
            b"1|PENDING|0:0|\n2|RUNNING|0:0|\n3|COMPLETED|0:0|\n4|CANCELLED by 42|0:15|\n5|TIMEOUT|1:0|\n",
        );

        assert_eq!(statuses.get("1"), Some(&SchedulerStatus::Queued));
        assert_eq!(statuses.get("2"), Some(&SchedulerStatus::Running));
        assert_eq!(statuses.get("3"), Some(&SchedulerStatus::Complete));
        assert_eq!(statuses.get("4"), Some(&SchedulerStatus::Cancelled));
        assert_eq!(
            statuses.get("5"),
            Some(&SchedulerStatus::Error {
                message: "Slurm job 5 failed: state=TIMEOUT, exit_code=1".to_string(),
            })
        );
    }
}
