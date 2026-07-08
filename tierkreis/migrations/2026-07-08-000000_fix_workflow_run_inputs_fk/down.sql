-- Revert to the original (broken) schema.
PRAGMA foreign_keys = OFF;

DROP TABLE IF EXISTS `workflow_run_inputs`;

CREATE TABLE `workflow_run_inputs` (
    `id`              INTEGER NOT NULL PRIMARY KEY,
    `workflow_run_id` TEXT    NOT NULL,
    `name`            TEXT    NOT NULL,
    `asset_kind`      TEXT    NOT NULL,
    `storage_name`    TEXT    NOT NULL,
    `asset_key`       TEXT    NOT NULL,
    FOREIGN KEY (`workflow_run_id`) REFERENCES `workflow_runs` (`id`)
);

PRAGMA foreign_keys = ON;
