use std::{
    io::{ErrorKind, SeekFrom},
    path::{Path, PathBuf},
};

use miette::{IntoDiagnostic, miette};
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncSeekExt},
};

use crate::{
    executor::{hpc::spec::shell_word, interface::TaskPlan},
    worker_logs::{CHUNK_BYTES, LogSource, TaskLogWriter, complete_utf8_prefix},
};

pub(super) struct HpcLogs {
    directory: PathBuf,
    writer: TaskLogWriter,
}

impl HpcLogs {
    pub(super) fn new(root: &Path, task: &TaskPlan, writer: TaskLogWriter) -> Self {
        let mut directory = root
            .join("worker-logs")
            .join(task.workflow_run_id.to_string())
            .join(task.attempt.to_string());
        directory.extend(task.loc.components().map(ToString::to_string));
        Self { directory, writer }
    }

    pub(super) async fn prepare(&self) -> miette::Result<()> {
        fs::create_dir_all(&self.directory).await.into_diagnostic()
    }

    pub(super) fn redirect(&self, command: &str) -> miette::Result<String> {
        let stdout = self.directory.join("stdout");
        let stderr = self.directory.join("stderr");
        let path_word = |path: &Path| {
            path.to_str()
                .map(shell_word)
                .ok_or_else(|| miette!("Non-UTF-8 HPC log path: {}", path.display()))
        };
        let stdout = path_word(&stdout)?;
        let stderr = path_word(&stderr)?;
        Ok(format!(
            "if (: >>{stdout} && : >>{stderr}); then\n  {command} >>{stdout} 2>>{stderr}\nelse\n  {command}\nfi"
        ))
    }

    pub(super) fn warn(&self, err: &miette::Report) {
        self.writer.warn(err);
    }

    pub(super) async fn collect(&self, finished: bool) {
        for source in [LogSource::Stdout, LogSource::Stderr] {
            let path = self.directory.join(source.as_str());
            if let Err(err) = self.collect_file(&path, source, finished).await {
                self.warn(&err);
            }
        }
        if finished {
            // Remove only the empty leaf. Capture for another node may share an ancestor.
            let _ = fs::remove_dir(&self.directory).await;
        }
    }

    async fn collect_file(
        &self,
        path: &Path,
        source: LogSource,
        finished: bool,
    ) -> miette::Result<()> {
        let mut file = match fs::File::open(path).await {
            Ok(file) => file,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
            Err(err) => return Err(err).into_diagnostic(),
        };

        if !self.writer.is_full() {
            let mut offset = self.writer.offset(source).await?;
            file.seek(SeekFrom::Start(u64::try_from(offset).into_diagnostic()?))
                .await
                .into_diagnostic()?;
            let mut buffer = [0; CHUNK_BYTES];
            let mut pending = 0;
            // Limit work per poll so other jobs can report their status and output.
            let reads = if finished { usize::MAX } else { 8 };
            for _ in 0..reads {
                let read = file.read(&mut buffer[pending..]).await.into_diagnostic()?;
                let len = pending + read;
                let end = if read == 0 && finished {
                    len
                } else {
                    complete_utf8_prefix(&buffer[..len])
                };
                if end > 0 {
                    let next = offset + i64::try_from(end).into_diagnostic()?;
                    let full = self
                        .writer
                        .append(
                            source,
                            &String::from_utf8_lossy(&buffer[..end]),
                            Some((offset, next)),
                        )
                        .await?;
                    offset = next;
                    if full {
                        break;
                    }
                }
                if read == 0 {
                    break;
                }
                buffer.copy_within(end..len, 0);
                pending = len - end;
            }
        }

        if finished {
            drop(file);
            fs::remove_file(path).await.into_diagnostic()?;
        }
        Ok(())
    }
}
