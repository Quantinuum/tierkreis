//! Slurm scheduler adapter.

use std::{path::PathBuf, time::Duration};

use futures::FutureExt;
use miette::{Context, IntoDiagnostic, Result, miette};
use tempfile::NamedTempFile;
use tokio::{process::Command, time::sleep};

use crate::executor::hpc::spec::ScriptTemplates;

use super::{JobSpec, SchedulerWrapper};

/// Slurm scheduler using `sbatch`, `sacct`, and `scancel`.
#[derive(Clone, Debug)]
pub struct SlurmWrapper {
    /// Submission command.
    pub sbatch: PathBuf,
    /// Accounting command.
    pub sacct: PathBuf,
    /// Cancellation command.
    pub scancel: PathBuf,
    /// Delay between accounting queries.
    pub poll_interval: Duration,
    templates: ScriptTemplates,
}

impl Default for SlurmWrapper {
    fn default() -> Self {
        Self {
            sbatch: "sbatch".into(),
            sacct: "sacct".into(),
            scancel: "scancel".into(),
            poll_interval: Duration::from_secs(1),
            templates: ScriptTemplates::default(),
        }
    }
}

impl SlurmWrapper {
    /// Construct a Slurm wrapper using shared submission templates.
    pub fn with_templates(templates: ScriptTemplates) -> Self {
        Self {
            templates,
            ..Self::default()
        }
    }
}

impl SchedulerWrapper for SlurmWrapper {
    fn submit(&self, spec: JobSpec) -> futures::future::BoxFuture<'_, Result<String>> {
        let scheduler = self.clone();
        async move {
            let script = NamedTempFile::new().into_diagnostic()?;
            std::fs::write(script.path(), scheduler.templates.render("slurm", &spec)?)
                .into_diagnostic()?;
            let output = Command::new(&scheduler.sbatch)
                .args(["--parsable"])
                .arg(script.path())
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

    fn wait(&self, job_id: String) -> futures::future::BoxFuture<'_, Result<()>> {
        let scheduler = self.clone();
        async move {
            loop {
                let output = Command::new(&scheduler.sacct)
                    .args(["-X", "-n", "-P", "-o", "State,ExitCode", "-j", &job_id])
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
                if let Some(line) = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .find(|line| !line.trim().is_empty())
                {
                    let mut fields = line.split('|');
                    let state = fields
                        .next()
                        .unwrap_or_default()
                        .split('+')
                        .next()
                        .unwrap_or_default();
                    let code = fields
                        .next()
                        .unwrap_or_default()
                        .split(':')
                        .next()
                        .unwrap_or("1");
                    if matches!(
                        state,
                        "COMPLETED"
                            | "FAILED"
                            | "CANCELLED"
                            | "TIMEOUT"
                            | "OUT_OF_MEMORY"
                            | "NODE_FAIL"
                    ) {
                        if state == "COMPLETED" && code == "0" {
                            return Ok(());
                        }
                        return Err(miette!(
                            "Slurm job {job_id} failed: state={state}, exit_code={code}"
                        ));
                    }
                }
                sleep(scheduler.poll_interval).await;
            }
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
