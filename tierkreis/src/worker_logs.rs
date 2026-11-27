//! Structured output from individual task executions.

use std::{
    collections::HashMap,
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{executor::interface::TaskPlan, location::Location, state::RuntimeState};

pub(crate) const CHUNK_BYTES: usize = 8 * 1024;
const TASK_BYTES: usize = 1024 * 1024;
const TRUNCATED: &str = "[Worker log truncated at 1 MiB]";

/// The origin of a worker log record.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum LogSource {
    /// The worker process's standard output.
    Stdout,
    /// The worker process's standard error.
    Stderr,
    /// A diagnostic from a Rust builtin.
    Builtin,
    /// A message supplied by a remote execution service.
    Provider,
}

impl LogSource {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
            Self::Builtin => "builtin",
            Self::Provider => "provider",
        }
    }
}

impl fmt::Display for LogSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The identity and metadata of one task's log.
#[derive(Clone, Debug)]
pub struct TaskLogKey {
    /// Workflow run UUID.
    pub run_id: Uuid,
    /// Workflow attempt number.
    pub attempt: u32,
    /// Full task-node location.
    pub location: Location,
    /// Configured executor name.
    pub executor: Arc<str>,
}

/// A chunk of output recorded for a task.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct WorkerLogRecord {
    /// Cursor in ingestion order.
    pub cursor: i64,
    /// Workflow run UUID.
    pub run_id: Uuid,
    /// Workflow attempt number.
    pub attempt: u32,
    /// Canonical task-node location.
    pub location: String,
    /// Time the runtime collected the message.
    pub collected_at: DateTime<Utc>,
    /// Configured executor name.
    pub executor: String,
    /// Origin of the message.
    pub source: LogSource,
    /// Captured output or diagnostic text.
    pub message: String,
}

/// A bounded query scoped to one run attempt.
#[derive(Debug)]
pub struct WorkerLogQuery {
    /// Workflow run UUID.
    pub run_id: Uuid,
    /// Workflow attempt number.
    pub attempt: u32,
    /// An exact node location, or all locations in the attempt.
    pub location: Option<Location>,
    /// Return records strictly after this cursor.
    pub after: i64,
    /// Return only records from this source when set.
    pub source: Option<LogSource>,
    /// Maximum number of records to return.
    pub limit: usize,
}

/// A batch of output and, for a file, the byte range it represents.
#[derive(Debug)]
pub struct WorkerLogBatch {
    /// Origin of the message.
    pub source: LogSource,
    /// Captured text, which may contain partial lines.
    pub message: String,
    /// Expected and next byte offsets for replayable file ingestion.
    pub offsets: Option<(i64, i64)>,
}

/// Log writers for the tasks dispatched to one executor.
#[derive(Clone, Debug)]
pub struct TaskLogSink {
    state: Arc<dyn RuntimeState>,
    executor: Arc<str>,
}

impl TaskLogSink {
    pub(crate) fn new(state: Arc<dyn RuntimeState>, executor: &str) -> Self {
        Self {
            state,
            executor: Arc::from(executor),
        }
    }

    /// Bind output to a task in this dispatch.
    #[must_use]
    pub fn for_task(&self, task: &TaskPlan) -> TaskLogWriter {
        TaskLogWriter {
            state: Arc::clone(&self.state),
            key: TaskLogKey {
                run_id: task.workflow_run_id,
                attempt: task.attempt,
                location: task.loc.clone(),
                executor: Arc::clone(&self.executor),
            },
            full: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// Best-effort writer bound to a task.
#[derive(Clone, Debug)]
pub struct TaskLogWriter {
    state: Arc<dyn RuntimeState>,
    key: TaskLogKey,
    full: Arc<AtomicBool>,
}

impl TaskLogWriter {
    /// Publish a message as one or more bounded records.
    pub async fn write(&self, source: LogSource, message: &str) {
        let mut remaining = message;
        while !remaining.is_empty() && !self.full.load(Ordering::Relaxed) {
            let end = remaining.floor_char_boundary(CHUNK_BYTES);
            let (chunk, rest) = remaining.split_at(end);
            if let Err(err) = self.append(source, chunk, None).await {
                self.warn(&err);
                break;
            }
            remaining = rest;
        }
    }

    pub(crate) fn is_full(&self) -> bool {
        self.full.load(Ordering::Relaxed)
    }

    pub(crate) async fn offset(&self, source: LogSource) -> miette::Result<i64> {
        self.state.worker_log_offset(&self.key, source).await
    }

    pub(crate) async fn append(
        &self,
        source: LogSource,
        message: &str,
        offsets: Option<(i64, i64)>,
    ) -> miette::Result<bool> {
        let full = self
            .state
            .append_worker_log(
                &self.key,
                WorkerLogBatch {
                    source,
                    message: message.to_owned(),
                    offsets,
                },
            )
            .await?;
        if full {
            self.full.store(true, Ordering::Relaxed);
        }
        Ok(full)
    }

    pub(crate) fn warn(&self, err: &miette::Report) {
        tracing::warn!(run_id = %self.key.run_id, attempt = self.key.attempt,
            loc = %self.key.location, executor = %self.key.executor, "Could not collect worker logs: {err:#}");
    }
}

#[derive(Default, Debug)]
pub(crate) struct LogBudget {
    pub bytes: usize,
    pub truncated: bool,
}

impl LogBudget {
    pub(crate) fn retain<'a>(&mut self, message: &'a str) -> impl Iterator<Item = &'a str> {
        let mut kept = "";
        let mut marker = "";
        if !self.truncated {
            let available = TASK_BYTES - TRUNCATED.len() - self.bytes;
            let end = message.floor_char_boundary(available);
            kept = &message[..end];
            self.bytes += end;
            if end < message.len() {
                self.truncated = true;
                marker = TRUNCATED;
            }
        }
        [kept, marker].into_iter().filter(|part| !part.is_empty())
    }
}

#[derive(Default, Debug)]
pub(crate) struct MemoryWorkerLogs {
    records: Vec<WorkerLogRecord>,
    budgets: HashMap<Location, LogBudget>,
    offsets: HashMap<(Location, LogSource), i64>,
}

impl MemoryWorkerLogs {
    pub(crate) fn append(&mut self, key: &TaskLogKey, batch: &WorkerLogBatch) -> bool {
        let budget = self.budgets.entry(key.location.clone()).or_default();
        if let Some((expected, next)) = batch.offsets {
            let offset = self
                .offsets
                .entry((key.location.clone(), batch.source))
                .or_default();
            if *offset != expected {
                return budget.truncated;
            }
            *offset = next;
        }
        for message in budget.retain(&batch.message) {
            self.records.push(WorkerLogRecord {
                cursor: self.records.last().map_or(1, |record| record.cursor + 1),
                run_id: key.run_id,
                attempt: key.attempt,
                location: key.location.to_string(),
                collected_at: Utc::now(),
                executor: key.executor.to_string(),
                source: batch.source,
                message: message.to_owned(),
            });
        }
        budget.truncated
    }

    pub(crate) fn offset(&self, key: &TaskLogKey, source: LogSource) -> i64 {
        self.offsets
            .get(&(key.location.clone(), source))
            .copied()
            .unwrap_or_default()
    }

    pub(crate) fn read(&self, query: &WorkerLogQuery) -> Vec<WorkerLogRecord> {
        let location = query.location.as_ref().map(ToString::to_string);
        let start = self
            .records
            .partition_point(|record| record.cursor <= query.after);
        self.records[start..]
            .iter()
            .filter(|record| {
                location.as_ref().is_none_or(|loc| loc == &record.location)
                    && query.source.is_none_or(|source| source == record.source)
            })
            .take(query.limit)
            .cloned()
            .collect()
    }
}

/// Length of the decodable prefix, holding back an incomplete trailing UTF-8 sequence.
pub(crate) fn complete_utf8_prefix(mut bytes: &[u8]) -> usize {
    let len = bytes.len();
    while let Err(err) = std::str::from_utf8(bytes) {
        if let Some(invalid) = err.error_len() {
            bytes = &bytes[err.valid_up_to() + invalid..];
        } else {
            return len - bytes.len() + err.valid_up_to();
        }
    }
    len
}

#[cfg(test)]
pub(crate) async fn test_log_sink(
    state: Arc<dyn RuntimeState>,
    executor: &str,
) -> miette::Result<(Arc<dyn crate::state::WorkflowRunState>, TaskLogSink)> {
    let id = state
        .save_workflow(
            None,
            crate::graph::WorkflowGraph::new(std::iter::empty::<String>()),
        )
        .await?;
    let run = state.new_workflow_run_state(id, HashMap::new()).await?;
    Ok((run, TaskLogSink::new(state, executor)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        event::WorkflowRunEvent,
        state::{InMemoryRuntimeState, SqliteRuntimeState},
    };

    #[rstest::rstest]
    #[case(false)]
    #[case(true)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn capture_is_scoped_bounded_and_replay_safe(#[case] sqlite: bool) -> miette::Result<()> {
        use miette::IntoDiagnostic as _;
        let dir = tempfile::tempdir().into_diagnostic()?;
        let path = dir.path().join("logs.sqlite");
        let url = path.to_str().unwrap();
        let state: Arc<dyn RuntimeState> = if sqlite {
            Arc::new(SqliteRuntimeState::try_new_with_url(url).await?)
        } else {
            Arc::new(InMemoryRuntimeState::new())
        };
        let (run, sink) = test_log_sink(Arc::clone(&state), "executor").await?;
        let mut task = TaskPlan {
            workflow_run_id: run.run_id(),
            loc: Location::new("N0.M1.N2")?,
            ..Default::default()
        };
        let writer = sink.for_task(&task);
        let data = "first";
        writer.append(LogSource::Stdout, data, Some((0, 5))).await?;
        writer.append(LogSource::Stdout, data, Some((0, 5))).await?;
        assert_eq!(writer.offset(LogSource::Stdout).await?, 5);
        writer
            .write(LogSource::Stdout, &"🙂".repeat(TASK_BYTES / 4))
            .await;
        writer.write(LogSource::Stderr, "discarded").await;

        let other = sink.for_task(&TaskPlan {
            loc: Location::new("N0.M2.N2")?,
            ..task.clone()
        });
        other.write(LogSource::Stderr, "other node").await;

        run.write(WorkflowRunEvent::Completed {}).await?;
        task.attempt = state.new_attempt(run.run_id()).await?.attempt();
        sink.for_task(&task)
            .write(LogSource::Stdout, "other attempt")
            .await;
        drop(writer);
        drop(other);
        drop(sink);
        drop(run);
        let state = if sqlite {
            drop(state);
            Arc::new(SqliteRuntimeState::try_new_with_url(url).await?) as Arc<dyn RuntimeState>
        } else {
            state
        };
        let read = |attempt| {
            state.read_worker_logs(WorkerLogQuery {
                run_id: task.workflow_run_id,
                attempt,
                location: Some(task.loc.clone()),
                source: None,
                after: 0,
                limit: 200,
            })
        };

        let original = read(0).await?.unwrap();
        assert_eq!(original.first().unwrap().message, data);
        assert_eq!(
            original
                .iter()
                .filter(|record| record.message == data)
                .count(),
            1
        );
        assert!(
            original
                .windows(2)
                .all(|records| records[0].cursor < records[1].cursor)
        );
        assert_eq!(original.last().unwrap().message, TRUNCATED);
        assert!(
            original
                .iter()
                .map(|record| record.message.len())
                .sum::<usize>()
                <= TASK_BYTES
        );
        assert!(
            original
                .iter()
                .all(|record| record.location == task.loc.to_string())
        );
        assert_eq!(
            read(task.attempt).await?.unwrap()[0].message,
            "other attempt"
        );
        assert!(read(task.attempt + 1).await?.is_none());
        Ok(())
    }
}
