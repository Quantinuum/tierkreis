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

use crate::runtime::RuntimeConfig;

/// Default file name
pub const CONFIG_FILE_NAME: &str = "tierkreis.toml";
/// Fallback env var
pub const CONFIG_ENV_VAR: &str = "TIERKREIS_CONFIG";
/// Name of the temporary files directory under the Tierkreis home directory.
pub const TMP_DIR_NAME: &str = "tmp";
/// Name of the default file asset storage directory un                                      der the Tierkreis home directory.
pub const ASSETS_DIR_NAME: &str = "assets";

/// Return the canonical Tierkreis home directory (`~/.tierkreis`), falling back
/// to `/tmp/.tierkreis` if the user's home directory cannot be determined.
#[must_use]
pub fn tierkreis_home_dir() -> PathBuf {
    home_dir().unwrap_or_else(|| "/tmp".into()).join(".tierkreis")
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

    let default = tierkreis_home_dir().join(CONFIG_FILE_NAME);
    default.is_file().then_some(default)
}

/// Create the default Tierkreis directories (`~/.tierkreis` and its `tmp` and
/// `assets` subdirectories) if they do not already exist.
///
/// # Errors
///
/// Returns an error if any of the directories cannot be created.
pub fn create_default_directories() -> miette::Result<()> {
    let home = tierkreis_home_dir();
    for dir in [home.join(TMP_DIR_NAME), home.join(ASSETS_DIR_NAME)] {
        std::fs::create_dir_all(&dir)
            .into_diagnostic()
            .wrap_err_with(|| format!("Failed to create directory at {}", dir.display()))?;
    }
    Ok(())
}

/// Write a default [`RuntimeConfig`] to the canonical config file location
/// (`~/.tierkreis/tierkreis.toml`) if one does not already exist there.
///
/// Returns the path to the config file, whether or not it was just created.
///
/// # Errors
///
/// Returns an error if the directory or file cannot be created, or if the
/// default config cannot be serialized.
pub fn create_default_config(config_path: Option<PathBuf>) -> miette::Result<PathBuf> {
    let path = config_path.unwrap_or_else(|| tierkreis_home_dir().join(CONFIG_FILE_NAME));
    if !path.is_file() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .into_diagnostic()
                .wrap_err_with(|| format!("Failed to create directory at {}", parent.display()))?;
        }
        let toml_str = RuntimeConfig::default().to_toml_string()?;
        std::fs::write(&path, toml_str)
            .into_diagnostic()
            .wrap_err_with(|| format!("Failed to write default config at {}", path.display()))?;
    }
    Ok(path)
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
        if let Some(path) = discover_config_path() {
            Self::from_file(&path)
        } else {
            tracing::warn!("No config file found, using default configuration.");
            Ok(Self::default())
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
}

#[cfg(test)]
mod tests {
    use crate::runtime::RuntimeConfig;

    #[test]
    fn default_config_round_trips_through_toml_file() {
        let config = RuntimeConfig::default();
        let toml_str = config
            .to_toml_string()
            .expect("Failed to serialize default config to TOML");
        let parsed_config = RuntimeConfig::from_toml_str(&toml_str)
            .expect("Failed to parse TOML string back to config");
        assert_eq!(config, parsed_config);
    }
}
