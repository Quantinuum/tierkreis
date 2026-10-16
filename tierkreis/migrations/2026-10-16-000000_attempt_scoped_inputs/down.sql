ALTER TABLE `workflow_run_inputs` RENAME TO `workflow_run_inputs_attempt_scoped`;

CREATE TABLE `workflow_run_inputs` (
	`id` INTEGER NOT NULL PRIMARY KEY,
	`workflow_run_id` TEXT NOT NULL,
	`name` TEXT NOT NULL,
	`asset_kind` TEXT NOT NULL,
	`storage_name` TEXT NOT NULL,
	`asset_key` TEXT NOT NULL,
	FOREIGN KEY (`workflow_run_id`) REFERENCES `workflow_runs` (`id`)
);

INSERT INTO `workflow_run_inputs` (
	`workflow_run_id`, `name`, `asset_kind`, `storage_name`, `asset_key`
)
SELECT
	`workflow_run_id`, `name`, `asset_kind`, `storage_name`, `asset_key`
FROM `workflow_run_inputs_attempt_scoped`
WHERE `attempt` = 0 AND `location` = '';

DROP TABLE `workflow_run_inputs_attempt_scoped`;
