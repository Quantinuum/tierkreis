//! PJSUB scheduler adapter.

use miette::{Context, IntoDiagnostic, Result, miette};
use std::{collections::HashMap, path::Path, path::PathBuf};
use tokio::process::Command;

use crate::executor::hpc::spec::{SchedulerStatus, ScriptTemplates};

use super::{JobSpec, SchedulerWrapper};

/// -z jid specified in the script so pjsub returns only the id on submission
fn parse_job_id(output: &[u8]) -> Option<String> {
    String::from_utf8_lossy(output)
        .split(|c: char| !c.is_ascii_digit())
        .rfind(|field| !field.is_empty())
        .map(str::to_string)
}

/*
From the docs:
ACC: Accepted job submission
RJT: Rejected job submission
QUE: Waiting for job execution
RNA: Acquiring resources required for job execution
RNP: Executing prologue
RUN: Executing job
RNE: Executing epilogue
RNO: Waiting for completion of job termination processing
EXT: Exited job end execution
CCL: Exited job execution by interruption
HLD: In fixed state due to users
ERR: In fixed state due to an error
*/
fn status_from_fields(job_id: &str, state: &str, exit_code: Option<i64>) -> SchedulerStatus {
    match state {
        "ACC" | "QUE" | "HLD" => SchedulerStatus::Queued,
        "RNA" | "RNP" | "RUN" | "RNE" | "RNO" => SchedulerStatus::Running,
        "EXT" if exit_code == Some(0) => SchedulerStatus::Complete,
        "CCL" => SchedulerStatus::Cancelled,
        "RJT" | "ERR" | "EXT" => SchedulerStatus::Error {
            message: format!(
                "PJSUB job {job_id} failed: state={state}, exit_code={}",
                exit_code.map_or_else(|| "unknown".to_string(), |code| code.to_string())
            ),
        },
        _ => SchedulerStatus::Error {
            message: format!("PJSUB job {job_id} failed: state={state}"),
        },
    }
}

fn parse_job_statuses(output: &[u8]) -> HashMap<String, SchedulerStatus> {
    String::from_utf8_lossy(output)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let mut fields = line.split('|').map(str::trim);
            let job_id = fields.next()?;
            if job_id.eq_ignore_ascii_case("jid") || job_id.eq_ignore_ascii_case("job_id") {
                return None;
            }
            let state = fields.next()?;
            let exit_code = fields.next().and_then(|field| field.parse::<i64>().ok());
            let status = status_from_fields(job_id, state, exit_code);
            Some((job_id.to_string(), status))
        })
        .collect()
}

/// PJSUB scheduler using `pjsub`, `pjstat`, and `pjdel`.
#[derive(Clone, Debug)]
pub struct PjsubWrapper {
    /// Submission command.
    pub pjsub: PathBuf,
    /// Status command.
    pub pjstat: PathBuf,
    /// Cancellation command.
    pub pjdel: PathBuf,
    templates: ScriptTemplates,
}

impl Default for PjsubWrapper {
    fn default() -> Self {
        Self {
            pjsub: "pjsub".into(),
            pjstat: "pjstat".into(),
            pjdel: "pjdel".into(),
            templates: ScriptTemplates::default(),
        }
    }
}

impl PjsubWrapper {
    /// Construct a PJSUB wrapper using shared submission templates.
    #[must_use]
    pub fn with_templates(templates: ScriptTemplates) -> Self {
        Self {
            templates,
            ..Self::default()
        }
    }
}

impl SchedulerWrapper for PjsubWrapper {
    async fn submit(&self, spec: JobSpec, script_path: &Path) -> Result<String> {
        let scheduler = self.clone();
        let script_path = script_path.to_path_buf();
        std::fs::write(&script_path, scheduler.templates.render("pjsub", &spec)?)
            .into_diagnostic()?;
        let output = Command::new(&scheduler.pjsub)
            .arg(&script_path)
            .output()
            .await
            .into_diagnostic()
            .wrap_err("Failed to invoke pjsub")?;
        if !output.status.success() {
            return Err(miette!(
                "pjsub failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        parse_job_id(&output.stdout).ok_or_else(|| miette!("pjsub returned no job id"))
    }

    async fn check(&self, job_ids: Vec<String>) -> Result<HashMap<String, SchedulerStatus>> {
        let scheduler = self.clone();
        if job_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let output = Command::new(&scheduler.pjstat)
            .args([
                "--choose",  // only output the following
                "jid,st,ec", // job id, state, job script exit code
                "--data",    // process output with below
                "--delimiter",
                "|", // Use | as delimiter
            ])
            .args(&job_ids)
            .output()
            .await
            .into_diagnostic()
            .wrap_err("Failed to invoke pjstat")?;
        if !output.status.success() {
            return Err(miette!(
                "pjstat failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(parse_job_statuses(&output.stdout))
    }

    async fn cancel(&self, job_id: String) -> Result<()> {
        let scheduler = self.clone();
        let status = Command::new(&scheduler.pjdel)
            .arg(job_id)
            .status()
            .await
            .into_diagnostic()?;
        status
            .success()
            .then_some(())
            .ok_or_else(|| miette!("pjdel failed: {status}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pjsub_job_id() {
        assert_eq!(
            parse_job_id(b"[INFO] PJM 0000 pjsub Job 12345 submitted.\n"),
            Some("12345".to_string())
        );
        assert_eq!(parse_job_id(b"12345\n"), Some("12345".to_string()));
    }

    #[test]
    fn parses_pjsub_job_statuses() {
        let statuses = parse_job_statuses(
            b"jid|st|ec
1|QUE|0
2|RUN|0
3|EXT|0
4|CCL|1
5|ERR|100",
        );

        assert_eq!(statuses.get("1"), Some(&SchedulerStatus::Queued));
        assert_eq!(statuses.get("2"), Some(&SchedulerStatus::Running));
        assert_eq!(statuses.get("3"), Some(&SchedulerStatus::Complete));
        assert_eq!(statuses.get("4"), Some(&SchedulerStatus::Cancelled));
        assert_eq!(
            statuses.get("5"),
            Some(&SchedulerStatus::Error {
                message: "PJSUB job 5 failed: state=ERR, exit_code=100".to_string(),
            })
        );
    }
}
