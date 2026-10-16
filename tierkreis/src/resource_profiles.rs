//! Site-specific resolution of portable execution contexts.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use miette::{Context, IntoDiagnostic, Result, miette};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Kubernetes like quantities (e.g., "100m", "2Gi")
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub(crate) enum Quantity {
    Integer(u64),
    Float(f64),
    Text(String),
}

pub(crate) fn to_f64(value: u64) -> Result<f64> {
    Ok(Into::<f64>::into(
        TryInto::<u32>::try_into(value).into_diagnostic()?,
    ))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn downcast(value: f64) -> u32 {
    (value).clamp(0.0, f64::from(u32::MAX)).trunc() as u32
}

impl Quantity {
    fn value(&self) -> Result<f64> {
        let value = match self {
            Self::Integer(number) => to_f64(*number)?,
            Self::Float(number) => *number,
            Self::Text(text) => {
                const SUFFIXES: [(&str, f64); 12] = [
                    ("Ki", 1024.0),
                    ("Mi", 1_048_576.0),
                    ("Gi", 1_073_741_824.0),
                    ("Ti", 1_099_511_627_776.0),
                    ("Pi", 1_125_899_906_842_624.0),
                    ("m", 0.001),
                    ("k", 1_000.0),
                    ("K", 1_000.0),
                    ("M", 1_000_000.0),
                    ("G", 1_000_000_000.0),
                    ("T", 1_000_000_000_000.0),
                    ("P", 1_000_000_000_000_000.0),
                ];
                let text = text.trim();
                let (number, multiplier) = SUFFIXES
                    .iter()
                    .find_map(|(suffix, multiplier)| {
                        text.strip_suffix(suffix)
                            .map(|number| (number, *multiplier))
                    })
                    .unwrap_or((text, 1.0));
                number
                    .trim()
                    .parse::<f64>()
                    .into_diagnostic()
                    .wrap_err_with(|| format!("Invalid resource quantity `{text}`"))?
                    * multiplier
            }
        };
        if !value.is_finite() || value < 0.0 {
            return Err(miette!("Resource quantity must be finite and nonnegative"));
        }
        Ok(value)
    }

    pub(crate) fn count(&self) -> Result<u32> {
        let count = self.value()?.ceil();
        if count > u32::MAX.into() {
            return Err(miette!("Resource quantity exceeds u32 range"));
        }
        Ok(downcast(count))
    }

    pub(crate) fn gib(&self) -> Result<u32> {
        let gib = (self.value()? / 1_073_741_824.0).ceil();
        if gib > u32::MAX.into() {
            return Err(miette!("Memory quantity exceeds u32 range"));
        }
        Ok(downcast(gib))
    }

    fn has_unit(&self) -> bool {
        matches!(self, Self::Text(text) if text.chars().last().is_some_and(char::is_alphabetic))
    }
}

/// Resource and scheduling settings for one execution context within a profile.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExecutionContext {
    executor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nodes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu: Option<Quantity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory: Option<Quantity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gpu: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_concurrency: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    walltime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    queue: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<toml::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mpi: Option<toml::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    container: Option<toml::Value>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    extra_scheduler_args: HashMap<String, Option<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    gres: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    qpus: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    modules: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    env: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    extras: HashMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    native: HashMap<String, HashMap<String, toml::Value>>,
}

impl ExecutionContext {
    fn validate(&self, name: &str) -> Result<()> {
        if let Some(cpu) = &self.cpu
            && cpu.value()? <= 0.0
        {
            return Err(miette!("Context `{name}` requires a positive cpu quantity"));
        }
        if self.gpu == Some(0) {
            return Err(miette!("Context `{name}` requires a positive gpu quantity"));
        }
        if self.nodes == Some(0) {
            return Err(miette!("Context `{name}` must request at least one node"));
        }
        if let Some(memory) = &self.memory {
            if !memory.has_unit() || memory.value()? <= 0.0 {
                return Err(miette!(
                    "Context `{name}` memory requires a positive unit, e.g. `16Gi`"
                ));
            }
            memory.gib()?;
        }
        if self.max_concurrency == Some(0) {
            return Err(miette!(
                "Context `{name}` must have a positive max_concurrency"
            ));
        }
        if let Some(walltime) = &self.walltime {
            let parts = walltime
                .split(':')
                .map(str::parse::<u32>)
                .collect::<std::result::Result<Vec<_>, _>>()
                .into_diagnostic()?;
            if parts.len() != 3 || parts[0] == 0 || parts[1] >= 60 || parts[2] >= 60 {
                return Err(miette!(
                    "Invalid walltime `{walltime}` in context `{name}`; expected HH:MM:SS"
                ));
            }
        }
        for key in self.extras.keys() {
            if key != "flags" {
                return Err(miette!(
                    "Unsupported extras key `{key}` in context `{name}`"
                ));
            }
        }
        Ok(())
    }
}

/// Executor placement and requirements for a single task.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResolvedContext {
    /// Name of the context selected for the task, if any.
    pub name: Option<String>,
    /// Executor selected by the profile, if any.
    pub executor: Option<String>,
    /// Resource requirements.
    pub resources: HashMap<String, Value>,
    /// Environment passed to the executor.
    pub environment: HashMap<String, Value>,
}

/// A selected profile's execution contexts.
#[derive(Debug, Default)]
pub struct ResourceProfiles {
    profiles: HashMap<String, ResolvedContext>,
}

impl ResourceProfiles {
    /// Load context files from the first matching profile directory in search-root priority order.
    ///
    /// # Errors
    /// Fails on a missing profile, malformed context, or invalid settings.
    pub fn load(roots: &[PathBuf], profile: &str) -> Result<Self> {
        if !valid_name(profile) {
            return Err(miette!("Invalid resource profile `{profile}`"));
        }
        let directory = roots
            .iter()
            .map(|root| root.join(profile))
            .find(|path| path.is_dir())
            .ok_or_else(|| miette!("Unknown resource profile `{profile}`"))?;

        let mut profiles = HashMap::new();
        for entry in std::fs::read_dir(&directory).into_diagnostic()? {
            let path = entry.into_diagnostic()?.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "toml")
                && path.file_name().is_some()
                && let Some(stem) = path.file_stem()
            {
                let name = stem.to_string_lossy().into_owned();
                let value: toml::Value = toml::from_str(&read(&path)?)
                    .into_diagnostic()
                    .wrap_err_with(|| format!("Invalid context `{name}` at {}", path.display()))?;
                profiles.insert(name.clone(), resolve_context(&name, value)?);
            }
        }

        if profiles
            .values()
            .any(|context| context.resources.contains_key("max_concurrency"))
        {
            tracing::warn!("max_concurrency is validated but not yet enforced by the orchestrator");
        }
        Ok(Self { profiles })
    }

    /// Resolve an explicitly requested context.
    ///
    /// # Errors
    /// Fails if the explicitly selected context is absent.
    pub fn resolve(
        &self,
        requested: Option<&str>,
        executor: Option<&str>,
    ) -> Result<ResolvedContext> {
        let mut context = match requested {
            Some(name) => {
                let mut context = self
                    .profiles
                    .get(name)
                    .cloned()
                    .ok_or_else(|| miette!("Unknown execution context `{name}`"))?;
                context.name = Some(name.to_owned());
                context
            }
            None => ResolvedContext::default(),
        };
        // Override the executor if explicitly requested.
        if let Some(executor) = executor {
            context.executor = Some(executor.to_owned());
        }
        let native_by_executor = context.resources.remove("native").unwrap_or(Value::Null);
        if let Some(native) = context
            .executor
            .as_deref()
            .and_then(|executor| native_by_executor.get(executor).cloned())
            && !native.as_object().is_some_and(serde_json::Map::is_empty)
        {
            context.resources.insert("native".into(), native);
        }
        Ok(context)
    }
}

/// Validate a context and build its executor payload.
///
/// # Errors
/// values that cannot be serialized for the executor.
fn resolve_context(name: &str, value: toml::Value) -> Result<ResolvedContext> {
    let context: ExecutionContext = value
        .try_into()
        .into_diagnostic()
        .wrap_err_with(|| format!("Invalid context `{name}`"))?;
    context.validate(name)?;

    let serde_json::Value::Object(mut fields) = serde_json::to_value(&context).into_diagnostic()?
    else {
        return Err(miette!("Execution context did not serialize as an object"));
    };
    let executor = fields
        .remove("executor")
        .and_then(|value| value.as_str().map(str::to_owned));
    let environment = fields
        .remove("env")
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default()
        .into_iter()
        .collect();
    // Escape hatch for native executor fields, e.g. --no-check-directory for pjsub
    let native_by_executor = fields.remove("native").unwrap_or(Value::Null);
    fields.retain(|_, value| !value.is_null());
    if !native_by_executor.is_null() {
        fields.insert("native".into(), native_by_executor);
    }

    Ok(ResolvedContext {
        name: None,
        executor,
        resources: fields.into_iter().collect(),
        environment,
    })
}

fn valid_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['/', '\\']) && name != "." && name != ".."
}

fn read(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .into_diagnostic()
        .wrap_err_with(|| format!("Could not read {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        sync::atomic::{AtomicUsize, Ordering},
    };

    static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let id = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "tierkreis-resource-profiles-{}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("create temporary test directory");
            Self(path)
        }

        fn profile(&self, name: &str) -> PathBuf {
            let path = self.0.join(name);
            fs::create_dir_all(&path).expect("create profile directory");
            path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_context(directory: &Path, name: &str, contents: &str) {
        fs::write(directory.join(format!("{name}.toml")), contents).expect("write test context");
    }

    #[test]
    fn parses_kubernetes_quantities() -> Result<()> {
        assert!((Quantity::Text("500m".into()).value()? - 0.5).abs() < f64::EPSILON);
        assert_eq!(Quantity::Text("1536Mi".into()).gib()?, 2);
        assert_eq!(Quantity::Text("2Gi".into()).gib()?, 2);
        assert_eq!(Quantity::Text("1.5".into()).count()?, 2);
        assert!(Quantity::Text("lots".into()).value().is_err());
        assert!(Quantity::Text("-1Gi".into()).value().is_err());
        assert!(Quantity::Float(f64::INFINITY).value().is_err());
        Ok(())
    }

    #[test]
    fn loads_a_context_from_its_file_and_ignores_environment_defaults() -> Result<()> {
        let root = TestDir::new();
        let profile = root.profile("cluster");
        write_context(
            &profile,
            "gpu",
            "executor = \"slurm\"\ncpu = \"500m\"\nmemory = \"1536Mi\"\n[native.slurm]\n\"--exclusive\" = true\n[env]\nTOKEN = \"present\"\n",
        );

        let profiles = ResourceProfiles::load(std::slice::from_ref(&root.0), "cluster")?;
        let context = profiles.resolve(Some("gpu"), None)?;
        assert_eq!(context.executor.as_deref(), Some("slurm"));
        assert_eq!(context.resources["cpu"], "500m");
        assert_eq!(context.resources["memory"], "1536Mi");
        assert_eq!(
            context.resources["native"],
            serde_json::json!({"--exclusive": true})
        );
        assert_eq!(context.environment["TOKEN"], "present");
        assert!(profiles.resolve(Some("alias"), None).is_err());
        Ok(())
    }

    #[test]
    fn no_requested_context_resolves_empty_and_unknown_context_fails() -> Result<()> {
        let root = TestDir::new();
        let profile = root.profile("cluster");
        write_context(&profile, "local", "cpu = 2\n");
        let profiles = ResourceProfiles::load(std::slice::from_ref(&root.0), "cluster")?;

        assert_eq!(profiles.resolve(None, None)?, ResolvedContext::default());
        assert!(profiles.resolve(Some("missing"), None).is_err());
        Ok(())
    }

    #[test]
    fn uses_only_the_first_matching_profile_directory() -> Result<()> {
        let high_priority = TestDir::new();
        let low_priority = TestDir::new();
        let high_profile = high_priority.profile("cluster");
        let low_profile = low_priority.profile("cluster");
        write_context(&high_profile, "shared", "cpu = 8\n");
        write_context(&low_profile, "shared", "memory = \"2Gi\"\n");
        write_context(&low_profile, "low_only", "cpu = 2\n");

        let profiles = ResourceProfiles::load(
            &[high_priority.0.clone(), low_priority.0.clone()],
            "cluster",
        )?;
        let context = profiles.resolve(Some("shared"), None)?;
        assert_eq!(context.resources["cpu"], 8);
        assert!(!context.resources.contains_key("memory"));
        assert!(profiles.resolve(Some("low_only"), None).is_err());
        Ok(())
    }
}
