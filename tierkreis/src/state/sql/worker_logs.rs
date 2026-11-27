use chrono::{DateTime, Utc};
use diesel::{
    OptionalExtension, QueryableByName, RunQueryDsl, SqliteConnection, sql_query,
    sql_types::{BigInt, Bool, Text},
};

use crate::worker_logs::{
    LogBudget, LogSource, TaskLogKey, WorkerLogBatch, WorkerLogQuery, WorkerLogRecord,
};

#[derive(QueryableByName)]
struct Offset {
    #[diesel(sql_type = BigInt)]
    byte_offset: i64,
}

pub(super) fn offset(
    conn: &mut SqliteConnection,
    key: &TaskLogKey,
    source: LogSource,
) -> diesel::QueryResult<i64> {
    sql_query("SELECT byte_offset FROM worker_log_offsets WHERE run_id = ? AND attempt = ? AND node_location = ? AND source = ?")
        .bind::<Text, _>(key.run_id.to_string())
        .bind::<BigInt, _>(i64::from(key.attempt))
        .bind::<Text, _>(key.location.to_string())
        .bind::<Text, _>(source.as_str())
        .get_result::<Offset>(conn)
        .optional()
        .map(|row| row.map_or(0, |row| row.byte_offset))
}

#[derive(QueryableByName)]
struct Budget {
    #[diesel(sql_type = BigInt)]
    bytes: i64,
    #[diesel(sql_type = Bool)]
    truncated: bool,
}

#[derive(QueryableByName)]
struct AttemptExists {
    #[diesel(sql_type = Bool)]
    present: bool,
}

fn attempt_exists(
    conn: &mut SqliteConnection,
    run_id: &str,
    attempt: i64,
) -> diesel::QueryResult<bool> {
    sql_query("SELECT EXISTS(SELECT 1 FROM workflow_run_attempts WHERE workflow_run_id = ? AND attempt = ?) AS present")
        .bind::<Text, _>(run_id)
        .bind::<BigInt, _>(attempt)
        .get_result::<AttemptExists>(conn)
        .map(|row| row.present)
}

pub(super) fn append(
    conn: &mut SqliteConnection,
    key: &TaskLogKey,
    batch: &WorkerLogBatch,
) -> diesel::QueryResult<bool> {
    conn.immediate_transaction(|conn| {
        let run_id = key.run_id.to_string();
        let attempt = i64::from(key.attempt);
        let location = key.location.to_string();
        let existing = sql_query("SELECT bytes, truncated FROM worker_log_budgets WHERE run_id = ? AND attempt = ? AND node_location = ?")
            .bind::<Text, _>(&run_id)
            .bind::<BigInt, _>(attempt)
            .bind::<Text, _>(&location)
            .get_result::<Budget>(conn)
            .optional()?;
        let mut budget = match existing {
            Some(row) => LogBudget {
                bytes: usize::try_from(row.bytes)
                    .map_err(|err| diesel::result::Error::DeserializationError(Box::new(err)))?,
                truncated: row.truncated,
            },
            None if attempt_exists(conn, &run_id, attempt)? => LogBudget::default(),
            None => return Err(diesel::result::Error::NotFound),
        };
        if let Some((expected, next)) = batch.offsets {
            if offset(conn, key, batch.source)? != expected {
                return Ok(budget.truncated);
            }
            sql_query("INSERT INTO worker_log_offsets (run_id, attempt, node_location, source, byte_offset) VALUES (?, ?, ?, ?, ?) ON CONFLICT DO UPDATE SET byte_offset = excluded.byte_offset")
                .bind::<Text, _>(&run_id)
                .bind::<BigInt, _>(attempt)
                .bind::<Text, _>(&location)
                .bind::<Text, _>(batch.source.as_str())
                .bind::<BigInt, _>(next)
                .execute(conn)?;
        }

        let collected_at = Utc::now().to_rfc3339();
        for message in budget.retain(&batch.message) {
            sql_query("INSERT INTO worker_logs (run_id, attempt, node_location, collected_at, executor, source, message) VALUES (?, ?, ?, ?, ?, ?, ?)")
                .bind::<Text, _>(&run_id)
                .bind::<BigInt, _>(attempt)
                .bind::<Text, _>(&location)
                .bind::<Text, _>(&collected_at)
                .bind::<Text, _>(key.executor.as_ref())
                .bind::<Text, _>(batch.source.as_str())
                .bind::<Text, _>(message)
                .execute(conn)?;
        }

        let bytes = i64::try_from(budget.bytes)
            .map_err(|err| diesel::result::Error::SerializationError(Box::new(err)))?;
        sql_query("INSERT INTO worker_log_budgets (run_id, attempt, node_location, bytes, truncated) VALUES (?, ?, ?, ?, ?) ON CONFLICT DO UPDATE SET bytes = excluded.bytes, truncated = excluded.truncated")
            .bind::<Text, _>(&run_id)
            .bind::<BigInt, _>(attempt)
            .bind::<Text, _>(&location)
            .bind::<BigInt, _>(bytes)
            .bind::<Bool, _>(budget.truncated)
            .execute(conn)?;
        Ok(budget.truncated)
    })
}

#[derive(QueryableByName)]
struct LogRow {
    #[diesel(sql_type = BigInt)]
    cursor: i64,
    #[diesel(sql_type = Text)]
    node_location: String,
    #[diesel(sql_type = Text)]
    collected_at: String,
    #[diesel(sql_type = Text)]
    executor: String,
    #[diesel(sql_type = Text)]
    source: String,
    #[diesel(sql_type = Text)]
    message: String,
}

pub(super) fn read(
    conn: &mut SqliteConnection,
    query: WorkerLogQuery,
) -> diesel::QueryResult<Option<Vec<WorkerLogRecord>>> {
    let run_id = query.run_id.to_string();
    let attempt = i64::from(query.attempt);
    if !attempt_exists(conn, &run_id, attempt)? {
        return Ok(None);
    }
    let mut sql = sql_query("SELECT cursor, node_location, collected_at, executor, source, message FROM worker_logs WHERE run_id = ? AND attempt = ? AND cursor > ?")
        .into_boxed::<diesel::sqlite::Sqlite>()
        .bind::<Text, _>(run_id)
        .bind::<BigInt, _>(attempt)
        .bind::<BigInt, _>(query.after);
    if let Some(location) = query.location {
        sql = sql
            .sql(" AND node_location = ?")
            .bind::<Text, _>(location.to_string());
    }
    if let Some(source) = query.source {
        sql = sql.sql(" AND source = ?").bind::<Text, _>(source.as_str());
    }
    sql.sql(" ORDER BY cursor LIMIT ?")
        .bind::<BigInt, _>(i64::try_from(query.limit).unwrap_or(i64::MAX))
        .load::<LogRow>(conn)?
        .into_iter()
        .map(|row| {
            let collected_at = DateTime::parse_from_rfc3339(&row.collected_at)
                .map_err(|err| diesel::result::Error::DeserializationError(Box::new(err)))?
                .to_utc();
            let source = match row.source.as_str() {
                "stdout" => LogSource::Stdout,
                "stderr" => LogSource::Stderr,
                "builtin" => LogSource::Builtin,
                "provider" => LogSource::Provider,
                other => {
                    return Err(diesel::result::Error::DeserializationError(
                        format!("Unknown worker log source {other}").into(),
                    ));
                }
            };
            Ok(WorkerLogRecord {
                cursor: row.cursor,
                run_id: query.run_id,
                attempt: query.attempt,
                location: row.node_location,
                collected_at,
                executor: row.executor,
                source,
                message: row.message,
            })
        })
        .collect::<diesel::QueryResult<_>>()
        .map(Some)
}
