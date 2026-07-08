-- The original workflow_run_inputs table had a FK that referenced only
-- workflow_runs(id), but the parent table has a composite PK (id, attempt).
-- SQLite (compiled with SQLITE_DEFAULT_FOREIGN_KEYS=1) raises a "foreign key
-- mismatch" error when inserting into a child table whose FK does not reference
-- all columns of the parent's PK or a UNIQUE index.
--
-- Fix: recreate the table with a correct composite FK.
-- Inputs are always stored against attempt=0 (the initial run creation) and
-- re-used for any later retry attempts.

PRAGMA foreign_keys = OFF;

DROP TABLE IF EXISTS `workflow_run_inputs`;

CREATE TABLE `workflow_run_inputs` (
    `id`                   INTEGER NOT NULL PRIMARY KEY,
    `workflow_run_id`      TEXT    NOT NULL,
    `workflow_run_attempt` INTEGER NOT NULL DEFAULT 0,
    `name`                 TEXT    NOT NULL,
    `asset_kind`           TEXT    NOT NULL,
    `storage_name`         TEXT    NOT NULL,
    `asset_key`            TEXT    NOT NULL,
    FOREIGN KEY (`workflow_run_id`, `workflow_run_attempt`)
        REFERENCES `workflow_runs` (`id`, `attempt`)
);

PRAGMA foreign_keys = ON;
