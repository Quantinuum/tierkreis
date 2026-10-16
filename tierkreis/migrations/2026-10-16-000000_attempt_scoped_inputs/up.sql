ALTER TABLE `workflow_run_inputs` RENAME TO `workflow_run_inputs_old`;

CREATE TABLE `workflow_run_inputs` (
	`id` INTEGER NOT NULL PRIMARY KEY,
	`workflow_run_id` TEXT NOT NULL,
	`attempt` INTEGER NOT NULL DEFAULT 0,
	`location` TEXT NOT NULL,
	`name` TEXT NOT NULL,
	`asset_kind` TEXT NOT NULL,
	`storage_name` TEXT NOT NULL,
	`asset_key` TEXT NOT NULL,
	UNIQUE (`workflow_run_id`, `attempt`, `location`, `name`),
	FOREIGN KEY (`workflow_run_id`, `attempt`)
		REFERENCES `workflow_run_attempts` (`workflow_run_id`, `attempt`)
);

INSERT INTO `workflow_run_inputs` (
	`id`, `workflow_run_id`, `attempt`, `location`, `name`,
	`asset_kind`, `storage_name`, `asset_key`
)
SELECT
	`id`, `workflow_run_id`, 0, '', `name`,
	`asset_kind`, `storage_name`, `asset_key`
FROM `workflow_run_inputs_old`;

DROP TABLE `workflow_run_inputs_old`;
