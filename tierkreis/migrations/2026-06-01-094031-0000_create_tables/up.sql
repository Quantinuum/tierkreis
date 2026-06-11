-- Your SQL goes here
CREATE TABLE `node_states`(
	`id` INTEGER NOT NULL PRIMARY KEY,
	`run_id` TEXT NOT NULL,
	`attempt` INTEGER NOT NULL,
	`node_location` TEXT NOT NULL,
	`scheduled_time` TIMESTAMP,
	`queued_time` TIMESTAMP,
	`running_time` TIMESTAMP,
	`complete_time` TIMESTAMP,
	`cancelled_time` TIMESTAMP,
	`error_time` TIMESTAMP,
	`cond` BOOLEAN,
	`loop_index` INTEGER,
	`map_completed` BLOB,
	`error` TEXT,
	`error_detail` TEXT,
	UNIQUE (`run_id`, `attempt`, `node_location`),
	FOREIGN KEY (`run_id`, `attempt`) REFERENCES `workflow_runs`(`id`, `attempt`)
);

CREATE TABLE `workflows`(
	`id` TEXT NOT NULL PRIMARY KEY,
	`name` TEXT,
	`created_time` TIMESTAMP
);

CREATE TABLE `node_outputs`(
	`id` INTEGER NOT NULL PRIMARY KEY,
	`node_state_id` INTEGER NOT NULL,
	`name` TEXT NOT NULL,
	`asset_kind` TEXT NOT NULL,
	`storage_name` TEXT NOT NULL,
	`asset_key` TEXT NOT NULL,
	FOREIGN KEY (`node_state_id`) REFERENCES `node_states`(`id`)
);

CREATE TABLE `workflow_runs`(
	`id` TEXT NOT NULL,
	`attempt` INTEGER NOT NULL,
	`workflow_id` TEXT NOT NULL,
	`run_metadata` BLOB NOT NULL CHECK (json_valid(run_metadata)),
	`status` TEXT,
	`started_time` TIMESTAMP,
	PRIMARY KEY (`id`, `attempt`),
	FOREIGN KEY (`workflow_id`) REFERENCES `workflows`(`id`)
);

