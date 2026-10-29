use std::{collections::HashMap, env::temp_dir, path::PathBuf};

use serde::{Deserialize, Serialize};

#[cfg(unix)]
use crate::executor::qrmi::QrmiExecutorConfig;
use crate::{
    config::{TKR_ENV_VAR, resource_profile_search_paths, tierkreis_home_dir},
    executor::{
        hpc::spec::HPCResourceSpec, nexus::NexusClientConfig,
        worker_command::ArchitectureWorkerCommands,
    },
    monitoring::LoggingConfig,
    resource_profiles::ResourceProfiles,
};

/// Configuration for the Tierkreis runtime.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub(super) asset_storage: HashMap<String, AssetStorageConfig>,
    pub(super) executors: HashMap<String, ExecutorConfig>,
    pub(super) runtime_state: RuntimeStateConfig,

    pub(super) default_storage_name: String,
    pub(super) default_executor_name: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub(super) worker_commands: ArchitectureWorkerCommands,
    pub(super) logging_config: Option<LoggingConfig>,
    #[serde(default)]
    pub(super) resources: ResourceSettings,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(super) struct ResourceSettings {
    pub(super) environment: Option<String>,
    #[serde(default)]
    pub(super) paths: Vec<PathBuf>,
}

impl ResourceSettings {
    pub(super) fn selected_profile(&self) -> Option<String> {
        self.environment
            .clone()
            .or_else(|| std::env::var(TKR_ENV_VAR).ok())
    }

    pub(super) fn load_profile(&self, profile: &str) -> miette::Result<ResourceProfiles> {
        ResourceProfiles::load(&resource_profile_search_paths(&self.paths), profile)
    }
}

impl RuntimeConfig {
    /// Select a deployment profile for this runtime configuration.
    #[must_use]
    pub fn with_profile(mut self, profile: String) -> Self {
        self.resources.environment = Some(profile);
        self
    }

    /// Override the configured logging level.
    pub fn set_log_level(&mut self, log_level: impl Into<String>) {
        let mut logging_config = self.logging_config.take().unwrap_or_default();
        logging_config.set_log_level(log_level);
        self.logging_config = Some(logging_config);
    }

    /// Construct a pre-defined config that keeps state in memory and uses temporary
    /// files to mediate with worker subprocesses.
    #[must_use]
    pub fn memory() -> Self {
        let asset_dir = temp_dir().join("tkr_temp_assets");
        Self {
            asset_storage: [
                ("memory".to_string(), AssetStorageConfig::Memory {}),
                (
                    "tempfile".to_string(),
                    AssetStorageConfig::File { asset_dir },
                ),
            ]
            .into_iter()
            .collect(),
            executors: [
                (
                    "memory".to_string(),
                    ExecutorConfig::Memory {
                        output_storage_name: "memory".to_string(),
                    },
                ),
                (
                    "subprocess".to_string(),
                    ExecutorConfig::Subprocess {
                        subprocess_storage_name: "tempfile".to_string(),
                        output_storage_name: "memory".to_string(),
                        worker_architecture: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
            runtime_state: RuntimeStateConfig::Memory {},
            default_storage_name: "memory".to_string(),
            default_executor_name: "memory".to_string(),
            worker_commands: ArchitectureWorkerCommands::new(),
            logging_config: Some(LoggingConfig::default()),
            resources: ResourceSettings::default(),
        }
    }

    /// Construct a pre-defined config that keeps state in an sqlite database
    /// in memory and can only run built-in tasks that also run in memory.
    #[must_use]
    pub fn sqlite_memory() -> Self {
        let mut config = Self::memory();
        config.runtime_state = RuntimeStateConfig::Sqlite {
            memory: true,
            url: None,
        };
        config
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        let asset_dir = tierkreis_home_dir().join("assets");
        Self {
            asset_storage: [
                ("memory".to_string(), AssetStorageConfig::Memory {}),
                ("file".to_string(), AssetStorageConfig::File { asset_dir }),
            ]
            .into_iter()
            .collect(),
            executors: [
                (
                    "memory".to_string(),
                    ExecutorConfig::Memory {
                        output_storage_name: "memory".to_string(),
                    },
                ),
                (
                    "subprocess".to_string(),
                    ExecutorConfig::Subprocess {
                        subprocess_storage_name: "file".to_string(),
                        output_storage_name: "file".to_string(),
                        worker_architecture: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
            runtime_state: RuntimeStateConfig::Memory {},
            default_storage_name: "file".to_string(),
            default_executor_name: "subprocess".to_string(),
            worker_commands: ArchitectureWorkerCommands::new(),
            logging_config: Some(LoggingConfig::default()),
            resources: ResourceSettings::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub(super) enum AssetStorageConfig {
    Memory {},
    File { asset_dir: PathBuf },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub(super) enum ExecutorConfig {
    Memory {
        output_storage_name: String,
    },
    Subprocess {
        subprocess_storage_name: String,
        output_storage_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        worker_architecture: Option<String>,
    },
    Nexus {
        client_config: NexusClientConfig,
        output_storage_name: String,
    },
    #[cfg(unix)]
    Qrmi {
        config: QrmiExecutorConfig,
        output_storage_name: String,
    },
    Hpc {
        hpc_storage_name: String,
        output_storage_name: String,
        scheduler: HpcSchedulerConfig,
        poll_interval_secs: Option<u64>,
        resources: HPCResourceSpec,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        worker_architecture: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum HpcSchedulerConfig {
    Pbs,
    Pjsub,
    Slurm,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub(super) enum RuntimeStateConfig {
    Memory {},
    Sqlite { memory: bool, url: Option<String> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_architecture_specific_worker_commands() -> miette::Result<()> {
        let config = RuntimeConfig::from_toml_str(
            r#"
default_storage_name = "file"
default_executor_name = "local"

[worker_commands.x86_64]
simulation_worker = ["uv", "run", "/workers/x86/main.py"]

[worker_commands.aarch64]
simulation_worker = ["uv", "run", "/workers/arm/main.py"]

[asset_storage.file]
type = "File"
asset_dir = "/tmp/assets"

[executors.local]
type = "Subprocess"
subprocess_storage_name = "file"
output_storage_name = "file"
worker_architecture = "x86_64"

[runtime_state]
type = "Memory"
"#,
        )?;

        assert_eq!(
            config.worker_commands["aarch64"]["simulation_worker"],
            ["uv", "run", "/workers/arm/main.py"]
        );
        assert!(matches!(
            &config.executors["local"],
            ExecutorConfig::Subprocess {
                worker_architecture: Some(architecture),
                ..
            } if architecture == "x86_64"
        ));
        Ok(())
    }
}
