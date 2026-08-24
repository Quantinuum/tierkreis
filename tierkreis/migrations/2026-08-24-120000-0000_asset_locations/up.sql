CREATE TABLE `asset_locations` (
    `asset_key` TEXT NOT NULL,
    `storage_name` TEXT NOT NULL,
    `location_type` TEXT NOT NULL,
    `schema_version` INTEGER NOT NULL,
    `data` BLOB NOT NULL CHECK (json_valid(`data`, 9)),
    PRIMARY KEY (`asset_key`, `storage_name`)
);
