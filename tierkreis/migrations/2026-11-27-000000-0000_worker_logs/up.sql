CREATE TABLE worker_logs (
    cursor INTEGER PRIMARY KEY,
    run_id TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    node_location TEXT NOT NULL,
    collected_at TEXT NOT NULL,
    executor TEXT NOT NULL,
    source TEXT NOT NULL,
    message TEXT NOT NULL,
    FOREIGN KEY (run_id, attempt) REFERENCES workflow_run_attempts(workflow_run_id, attempt)
);
CREATE INDEX worker_logs_by_run ON worker_logs(run_id, attempt, cursor);
CREATE INDEX worker_logs_by_node ON worker_logs(run_id, attempt, node_location, cursor);

CREATE TABLE worker_log_budgets (
    run_id TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    node_location TEXT NOT NULL,
    bytes INTEGER NOT NULL,
    truncated BOOLEAN NOT NULL,
    PRIMARY KEY (run_id, attempt, node_location),
    FOREIGN KEY (run_id, attempt) REFERENCES workflow_run_attempts(workflow_run_id, attempt)
);

CREATE TABLE worker_log_offsets (
    run_id TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    node_location TEXT NOT NULL,
    source TEXT NOT NULL,
    byte_offset INTEGER NOT NULL,
    PRIMARY KEY (run_id, attempt, node_location, source),
    FOREIGN KEY (run_id, attempt) REFERENCES workflow_run_attempts(workflow_run_id, attempt)
);
