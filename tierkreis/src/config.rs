/*!
Handling of project configuration files.
Discovery and (de)serialization of the [`RuntimeConfig`](crate::runtime::RuntimeConfig)
from a TOML file on disk.
*/
use std::{
    env,
    env::home_dir,
    path::{Path, PathBuf},
};

use miette::{Context, IntoDiagnostic};
use serde::{Deserialize, Serialize};

use crate::runtime::RuntimeConfig;

/// Default file name
pub const CONFIG_FILE_NAME: &str = "tierkreis.toml";
/// Fallback env var
pub const CONFIG_ENV_VAR: &str = "TIERKREIS_CONFIG";

/// Requirements or capabilities associated with a named resource.
#[derive(Serialize, Deserialize)]
pub struct ResourceConfig {
    /// Number of compute nodes.
    pub nodes: Option<u32>,
    /// CPU cores per node, allowing fractional quotas.
    pub cpu_cores: Option<f64>,
    /// Memory per node, retained as a human-readable quantity such as `16G`.
    pub memory: Option<String>,
    /// GPU requirement or capacity.
    pub gpu: Option<GpuConfig>,
    /// Named QPUs and their optional qubit counts.
    pub qpu: Option<Vec<QpuConfig>>,
    /// Generic scheduler resources such as `sharedtmp-size=64Gi`.
    pub gres: Option<Vec<String>>,
    /// MPI requirements.
    pub mpi: Option<MpiConfig>,
    /// Hard task timeout, for example `2h`.
    pub timeout: Option<String>,
    /// Optional executor name to constrain placement.
    pub executor: Option<String>,
}

/// GPU requirement or capacity.
#[derive(Serialize, Deserialize)]
pub struct GpuConfig {
    /// Number of GPUs.
    pub count: u32,
    /// Optional GPU vendor, such as `nvidia`.
    pub vendor: Option<String>,
}

/// Named QPU requirement or capacity.
#[derive(Serialize, Deserialize)]
pub struct QpuConfig {
    /// QPU name.
    pub name: String,
    /// Optional number of available qubits.
    pub qubits: Option<u32>,
}

/// MPI requirement or capacity.
#[derive(Serialize, Deserialize)]
pub struct MpiConfig {
    /// Whether MPI is enabled.
    pub enabled: bool,
    /// MPI processes per node.
    pub processes_per_node: Option<u32>,
}

/// Locate a `RuntimeConfig` TOML file, searching:
///
/// 1. [`CONFIG_FILE_NAME`] in the current directory or an ancestor directory.
/// 2. The path in the [`CONFIG_ENV_VAR`] environment variable.
/// 3. The canonical location returned by [`canonical_config_path`].
///
/// Returns `None` if no config file exists at any of these locations.
#[must_use]
pub fn discover_config_path() -> Option<PathBuf> {
    if let Ok(cwd) = env::current_dir()
        && let Some(path) = find_in_ancestors(&cwd, CONFIG_FILE_NAME)
    {
        return Some(path);
    }

    if let Ok(path) = env::var(CONFIG_ENV_VAR) {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    let default = home_dir()
        .unwrap_or_else(|| "/tmp".into())
        .join(".tierkreis")
        .join(CONFIG_FILE_NAME);
    default.is_file().then_some(default)
}

/// Walk up from `start`, looking for a file named `name` in each directory.
fn find_in_ancestors(start: &Path, name: &str) -> Option<PathBuf> {
    let mut dir = Some(start);
    while let Some(d) = dir {
        let candidate = d.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
        dir = d.parent();
    }
    None
}

impl RuntimeConfig {
    /// Load the runtime config from the file found by [`discover_config_path`],
    /// falling back to [`RuntimeConfig::default`] if no config file is found.
    ///
    /// # Errors
    ///
    /// Returns an error if a config file is found but cannot be read or parsed.
    pub fn load() -> miette::Result<Self> {
        match discover_config_path() {
            Some(path) => Self::from_file(&path),
            None => Ok(Self::default()),
        }
    }

    /// Read and parse a `RuntimeConfig` from the TOML file at `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or its contents are not a
    /// valid `RuntimeConfig` TOML document.
    pub fn from_file(path: &Path) -> miette::Result<Self> {
        let contents = std::fs::read_to_string(path)
            .into_diagnostic()
            .wrap_err_with(|| format!("Failed to read config file at {}", path.display()))?;
        Self::from_toml_str(&contents)
            .wrap_err_with(|| format!("Failed to parse config file at {}", path.display()))
    }

    /// Parse a `RuntimeConfig` from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns an error if `contents` is not a valid `RuntimeConfig` TOML document.
    pub fn from_toml_str(contents: &str) -> miette::Result<Self> {
        toml::from_str(contents).into_diagnostic()
    }

    /// Serialize this `RuntimeConfig` to a pretty-printed TOML string.
    ///
    /// # Errors
    ///
    /// Returns an error if the config cannot be represented as TOML.
    pub fn to_toml_string(&self) -> miette::Result<String> {
        toml::to_string_pretty(self).into_diagnostic()
    }

    /// Serialize this `RuntimeConfig` to a TOML file at `path`, creating
    /// parent directories as needed.
    ///
    /// # Errors
    ///
    /// Returns an error if the config cannot be serialized or the file cannot be written.
    pub fn save(&self, path: &Path) -> miette::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).into_diagnostic()?;
        }
        std::fs::write(path, self.to_toml_string()?).into_diagnostic()
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::RuntimeConfig;

    #[test]
    fn default_config_round_trips_through_toml_file() {
        let config = RuntimeConfig::default();
        let file = tempfile::NamedTempFile::new().unwrap();

        config.save(file.path()).unwrap();
        let loaded = RuntimeConfig::from_file(file.path()).unwrap();

        // `RuntimeConfig` doesn't implement `PartialEq` and its `HashMap`
        // fields don't serialize in a stable order, so compare the parsed
        // TOML values rather than the raw strings.
        let original: toml::Value = toml::from_str(&config.to_toml_string().unwrap()).unwrap();
        let round_tripped: toml::Value = toml::from_str(&loaded.to_toml_string().unwrap()).unwrap();
        assert_eq!(original, round_tripped);
    }

    #[test]
    fn resource_requirements_deserialize_from_toml() {
        let config = RuntimeConfig::from_toml_str(
            r#"
version = "1.0"
default_storage_name = "memory"
default_executor_name = "local1"

[asset_storage.memory]
type = "Memory"

[executors.local1]
type = "Memory"
output_storage_name = "memory"

[runtime_state]
type = "Memory"

[resources.large]
nodes = 16
cpu_cores = 4.0
memory = "16G"
timeout = "2h"
executor = "local1"

[resources.large.gpu]
count = 2
vendor = "nvidia"

[[resources.large.qpu]]
name = "helios"
qubits = 98

[resources.large.mpi]
enabled = true
processes_per_node = 4
"#,
        )
        .unwrap();

        let resource = config.resources.get("large").unwrap();
        assert_eq!(resource.nodes, Some(16));
        assert_eq!(resource.cpu_cores, Some(4.0));
        assert_eq!(resource.memory.as_deref(), Some("16G"));
        assert_eq!(resource.gpu.as_ref().unwrap().count, 2);
        assert_eq!(resource.qpu.as_ref().unwrap()[0].qubits, Some(98));
        assert_eq!(resource.mpi.as_ref().unwrap().processes_per_node, Some(4));
    }
}
