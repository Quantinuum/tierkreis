ALTER TABLE `node_states`
    ADD COLUMN `stdout_log_path` TEXT DEFAULT NULL;

ALTER TABLE `node_states`
    ADD COLUMN `stderr_log_path` TEXT DEFAULT NULL;
