-- Your SQL goes here
CREATE TABLE `node_states`(
	`id` INTEGER NOT NULL PRIMARY KEY,
	`run_id` TEXT NOT NULL,
	`attempt` INTEGER NOT NULL,
	`node_location` TEXT NOT NULL,
	`scheduled_time` TIMESTAMP DEFAULT NULL,
	`queued_time` TIMESTAMP DEFAULT NULL,
	`running_time` TIMESTAMP DEFAULT NULL,
	`complete_time` TIMESTAMP DEFAULT NULL,
	`cancelled_time` TIMESTAMP DEFAULT NULL,
	`error_time` TIMESTAMP DEFAULT NULL,
	`cond` BOOLEAN DEFAULT NULL,
	`loop_index` INTEGER DEFAULT NULL,
	`map_size` INTEGER DEFAULT NULL,
	`map_completed` BLOB DEFAULT NULL,
	`error` TEXT DEFAULT NULL,
	`error_detail` TEXT DEFAULT NULL,
	UNIQUE (`run_id`, `attempt`, `node_location`),
	FOREIGN KEY (`run_id`, `attempt`)
		REFERENCES `workflow_run_attempts`(`workflow_run_id`, `attempt`)
);

CREATE TABLE `workflows`(
	`id` TEXT NOT NULL PRIMARY KEY,
	`name` TEXT,
	`created_time` TIMESTAMP,
	`definition` BLOB NOT NULL
);

CREATE TABLE `node_outputs`(
	`id` INTEGER NOT NULL PRIMARY KEY,
	`node_state_id` INTEGER NOT NULL,
	`name` TEXT NOT NULL,
	`asset_kind` TEXT NOT NULL,
	`storage_name` TEXT NOT NULL,
	`asset_key` TEXT NOT NULL,
	UNIQUE (`node_state_id`, `name`),
	FOREIGN KEY (`node_state_id`) REFERENCES `node_states`(`id`)
);

CREATE TABLE `workflow_runs`(
	`id` TEXT NOT NULL PRIMARY KEY,
	`workflow_id` TEXT NOT NULL,
	FOREIGN KEY (`workflow_id`) REFERENCES `workflows`(`id`)
);

CREATE TABLE `workflow_run_attempts`(
	`id` INTEGER NOT NULL PRIMARY KEY,
	`workflow_run_id` TEXT NOT NULL,
	`attempt` INTEGER NOT NULL DEFAULT 0,
	`run_metadata` BLOB NOT NULL CHECK (json_valid(run_metadata, 8)) DEFAULT (jsonb('{}')),
	`status` TEXT DEFAULT NULL,
	`started_time` TIMESTAMP DEFAULT NULL,
	UNIQUE (`workflow_run_id`, `attempt`),
	FOREIGN KEY (`workflow_run_id`) REFERENCES `workflow_runs`(`id`)
);

CREATE TABLE `workflow_run_inputs` (
	`id` INTEGER NOT NULL PRIMARY KEY,
	`workflow_run_id` TEXT NOT NULL,
	`name` TEXT NOT NULL,
	`asset_kind` TEXT NOT NULL,
	`storage_name` TEXT NOT NULL,
	`asset_key` TEXT NOT NULL,
	FOREIGN KEY (`workflow_run_id`) REFERENCES `workflow_runs` (`id`)
);

CREATE TABLE `executor_debug` (
	`id` INTEGER NOT NULL PRIMARY KEY,
	`node_state_id` INTEGER NOT NULL,
	`executor_name` TEXT NOT NULL,
	`worker_name` TEXT NOT NULL,
	`task_name` TEXT NOT NULL,
	`resources` BLOB NOT NULL CHECK (json_valid(resources, 8)) DEFAULT (jsonb('{}')),
	`environment` BLOB NOT NULL CHECK (json_valid(environment, 8)) DEFAULT (jsonb('{}')),
	`internal_id` TEXT DEFAULT NULL,
	UNIQUE (`node_state_id`),
	FOREIGN KEY (`node_state_id`)
		REFERENCES `node_states`(`id`)
);
