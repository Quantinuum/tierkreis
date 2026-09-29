//! Site-specific resolution of portable execution contexts.

use std::{
    collections::{HashMap, HashSet},
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

impl Quantity {
    #[allow(clippy::cast_precision_loss)]
    fn value(&self) -> Result<f64> {
        let value = match self {
            Self::Integer(number) => *number as f64,
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

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub(crate) fn count(&self) -> Result<u32> {
        let count = self.value()?.ceil();
        if count > f64::from(u32::MAX) {
            return Err(miette!("Resource quantity exceeds u32 range"));
        }
        Ok(count as u32)
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub(crate) fn gib(&self) -> Result<u32> {
        let gib = (self.value()? / 1_073_741_824.0).ceil();
        if gib > f64::from(u32::MAX) {
            return Err(miette!("Memory quantity exceeds u32 range"));
        }
        Ok(gib as u32)
    }

    fn has_unit(&self) -> bool {
        matches!(self, Self::Text(text) if text.chars().last().is_some_and(char::is_alphabetic))
    }
}

/// Profile configuration loaded from `_env.toml`.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileConfig {
    extends: Option<String>,
    #[serde(default)]
    r#abstract: bool,
    /// Context used when neither the task nor its worker selects one.
    #[serde(default)]
    default_context: Option<String>,
    /// Maps context names to their corresponding configuration files.
    #[serde(default)]
    contexts: HashMap<String, String>,
    /// Maps worker names to their default execution contexts.
    #[serde(default)]
    workers: HashMap<String, WorkerDefault>,
}

/// Binds all tasks of a worker to a specific execution context.
/// Overwritten on a per-task basis if the task specifies a different execution context.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkerDefault {
    context: String,
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
    gpu: Option<Quantity>,
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
        for (key, quantity) in [("cpu", &self.cpu), ("gpu", &self.gpu)] {
            if let Some(quantity) = quantity
                && quantity.value()? <= 0.0
            {
                return Err(miette!(
                    "Context `{name}` requires a positive {key} quantity"
                ));
            }
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

/// A selected profile's contexts and worker defaults.
#[derive(Debug, Default)]
pub struct ResourceProfiles {
    default: Option<String>,
    workers: HashMap<String, String>,
    /// Final mapping of context names to their resolved execution contexts.
    profiles: HashMap<String, ResolvedContext>,
}

impl ResourceProfiles {
    /// Load a profile from search roots in priority order.
    ///
    /// # Errors
    /// Fails on missing, malformed, cyclic, or ambiguous contexts.
    pub fn load(roots: &[PathBuf], profile: &str) -> Result<Self> {
        // Identify the profile configuration
        let chain = load_profile(roots, profile)?;

        let mut default = None;
        let mut workers = HashMap::new();
        let mut raw: HashMap<String, toml::Value> = HashMap::new();

        // Aggregate raw context configurations from all layers in the profile chain.
        for (directory, config) in chain.iter().flatten() {
            if let Some(name) = &config.default_context {
                default = Some(name.clone());
            }
            workers.extend(
                config
                    .workers
                    .iter()
                    .map(|(worker, value)| (worker.clone(), value.context.clone())),
            );
            load_context_files(directory, config, &mut raw)?;
        }

        // Resolve all raw context configurations into their final form.
        let profiles = raw
            .keys()
            .map(|name| Ok((name.clone(), resolve_context(name, &raw)?)))
            .collect::<Result<HashMap<_, _>>>()?;

        // Check that the default and worker contexts exist in the resolved profiles.
        for reference in default.iter().chain(workers.values()) {
            if !profiles.contains_key(reference) {
                return Err(miette!(
                    "Unknown default context `{reference}` in profile `{profile}`"
                ));
            }
        }
        // TODO
        if profiles
            .values()
            .any(|context| context.resources.contains_key("max_concurrency"))
        {
            tracing::warn!("max_concurrency is validated but not yet enforced by the orchestrator");
        }
        Ok(Self {
            default,
            workers,
            profiles,
        })
    }

    /// Resolve task, worker, and profile defaults in that order.
    ///
    /// # Errors
    /// Fails if an explicitly selected context is absent.
    pub fn resolve(&self, worker: &str, requested: Option<&str>) -> Result<ResolvedContext> {
        let name = requested
            .or_else(|| self.workers.get(worker).map(String::as_str))
            .or(self.default.as_deref());
        match name {
            Some(name) => {
                let mut context = self.profiles.get(name).cloned().ok_or_else(|| {
                    miette!("Unknown execution context `{name}` for worker `{worker}`")
                })?;
                context.name = Some(name.to_owned());
                Ok(context)
            }
            None => Ok(ResolvedContext::default()),
        }
    }
}

/// Loads the profile inheritance chain for the specified profile.
fn load_profile(roots: &[PathBuf], profile: &str) -> Result<Vec<Vec<(PathBuf, ProfileConfig)>>> {
    let mut chain = Vec::new();
    let mut visited = HashSet::new();
    let mut current = profile.to_string();
    loop {
        if !valid_name(&current) || !visited.insert(current.clone()) {
            return Err(miette!("Invalid or cyclic profile inheritance: {current}"));
        }
        // Load every root layer for the current profile.
        let profile_envs = roots
            .iter()
            .rev()
            .map(|root| root.join(&current))
            .filter(|path| path.is_dir())
            .map(|directory| {
                let env_path = directory.join("_env.toml");
                let config = if env_path.is_file() {
                    toml::from_str(&read(&env_path)?)
                        .into_diagnostic()
                        .wrap_err_with(|| format!("Invalid {}", env_path.display()))?
                } else {
                    ProfileConfig::default()
                };
                Ok((directory, config))
            })
            .collect::<Result<Vec<_>>>()?;
        if profile_envs.is_empty() {
            return Err(miette!("Unknown resource profile `{current}`"));
        }
        if current == profile && profile_envs.iter().any(|(_, config)| config.r#abstract) {
            return Err(miette!("Profile `{current}` is abstract"));
        }
        // Determine the parent profile, if any.
        let parent = profile_envs
            .iter()
            .rev()
            .find_map(|(_, config)| config.extends.clone());
        chain.push(profile_envs);
        match parent {
            Some(name) => current = name,
            None => break,
        }
    }
    chain.reverse();
    Ok(chain)
}

/// Read and merge a profile's context files into the accumulated set.
///
/// Implicit file names and aliases from `config.contexts` share one namespace;
/// later profiles override earlier values.
///
/// # Errors
/// Returns an error for invalid or escaping aliases, conflicting context names,
/// or unreadable or malformed context files.
fn load_context_files(
    directory: &Path,
    config: &ProfileConfig,
    contexts: &mut HashMap<String, toml::Value>,
) -> Result<()> {
    let mut files = HashMap::new();
    // All named files
    for entry in std::fs::read_dir(directory).into_diagnostic()? {
        let path = entry.into_diagnostic()?.path();
        if path.extension().is_some_and(|ext| ext == "toml")
            && path.file_name().is_some_and(|name| name != "_env.toml")
            && let Some(stem) = path.file_stem()
        {
            files.insert(stem.to_string_lossy().into_owned(), path);
        }
    }

    // Apply explicitly mapped context names.
    let mut mapped = Vec::new();
    for (name, relative) in &config.contexts {
        if !valid_name(name) || relative.split('/').any(|part| part == "..") {
            return Err(miette!("Invalid context mapping `{name}`: `{relative}`"));
        }
        let path = directory.join(relative);
        let canonical = path.canonicalize().into_diagnostic()?;
        if !canonical.starts_with(directory.canonicalize().into_diagnostic()?) {
            return Err(miette!("Context `{name}` escapes its profile directory"));
        }
        mapped.push((name, path));
    }

    files.retain(|_, path| !mapped.iter().any(|(_, target)| target == path));
    for (name, path) in mapped {
        if let Some(existing) = files.get(name)
            && existing != &path
        {
            return Err(miette!(
                "Context `{name}` is defined by both {} and {}",
                existing.display(),
                path.display()
            ));
        }
        files.insert(name.clone(), path);
    }

    for (name, path) in files {
        let value: toml::Value = toml::from_str(&read(&path)?)
            .into_diagnostic()
            .wrap_err_with(|| format!("Invalid context `{name}` at {}", path.display()))?;
        match contexts.get_mut(&name) {
            Some(parent) => merge(parent, value),
            None => {
                contexts.insert(name, value);
            }
        }
    }
    Ok(())
}

/// Resolve a named context's inheritance, validate it, and build its executor payload.
///
/// # Errors
/// Returns an error for an unknown or cyclic context, invalid settings, or
/// values that cannot be serialized for the executor.
fn resolve_context(name: &str, raw: &HashMap<String, toml::Value>) -> Result<ResolvedContext> {
    let resolved = resolve(name, raw, &mut Vec::new())?;
    let context: ExecutionContext = resolved
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
    let native_by_executor = fields.remove("native").unwrap_or(Value::Null);
    let native = executor
        .as_deref()
        .and_then(|executor| native_by_executor.get(executor).cloned());
    fields.retain(|_, value| !value.is_null());
    if let Some(native) = native
        && !native.as_object().is_some_and(serde_json::Map::is_empty)
    {
        fields.insert("native".into(), native);
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

/// Resolve inherited context values.
/// TODO: Is this a good idea?
/// Especial the HPC context has some favorable use cases.
fn resolve(
    name: &str,
    raw: &HashMap<String, toml::Value>,
    stack: &mut Vec<String>,
) -> Result<toml::Value> {
    if stack.iter().any(|item| item == name) {
        return Err(miette!(
            "Cyclic context inheritance: {} -> {name}",
            stack.join(" -> ")
        ));
    }
    let mut child = raw
        .get(name)
        .cloned()
        .ok_or_else(|| miette!("Unknown parent context `{name}`"))?;
    let parent = child
        .get("extends")
        .and_then(toml::Value::as_str)
        .map(str::to_owned);
    if let Some(parent) = parent {
        stack.push(name.to_owned());
        let mut inherited = resolve(&parent, raw, stack)?;
        stack.pop();
        child
            .as_table_mut()
            .ok_or_else(|| miette!("Context `{name}` must be a table"))?
            .remove("extends");
        merge(&mut inherited, child);
        Ok(inherited)
    } else {
        Ok(child)
    }
}

/// Merge child context into parent context, with child values taking precedence.
fn merge(parent: &mut toml::Value, child: toml::Value) {
    match (parent, child) {
        (toml::Value::Table(parent), toml::Value::Table(child)) => {
            for (key, value) in child {
                if let Some(old) = parent.get_mut(&key) {
                    merge(old, value);
                } else {
                    parent.insert(key, value);
                }
            }
        }
        (toml::Value::Array(parent), toml::Value::Array(child)) => {
            for value in child {
                // If the value is a string starting with '!', remove the corresponding item from the parent array.
                // e.g. to remove a preselected HPC module
                if let Some(remove) = value.as_str().and_then(|text| text.strip_prefix('!')) {
                    parent.retain(|item| item.as_str() != Some(remove));
                } else if !parent.contains(&value) {
                    parent.push(value);
                }
            }
        }
        (parent, child) => *parent = child,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile_fixtures() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/resource_profiles")
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
    fn selected_executor_receives_inherited_native_settings() -> Result<()> {
        let profiles = ResourceProfiles::load(&[profile_fixtures()], "cluster")?;
        let context = profiles.resolve("tkr-qulacs-worker", None)?;
        assert_eq!(context.name.as_deref(), Some("gpu_large"));
        assert_eq!(context.executor.as_deref(), Some("slurm"));
        assert_eq!(context.resources["cpu"], "500m");
        assert_eq!(context.resources["memory"], "1536Mi");
        assert_eq!(context.resources["gpu"], "500m");
        assert_eq!(context.resources["max_concurrency"], 2);
        assert_eq!(
            context.resources["native"],
            serde_json::json!({"--exclusive": true, "--partition": "gpu"})
        );
        assert_eq!(
            context.resources["modules"],
            serde_json::json!(["openmpi/5", "cuda/12"])
        );
        let context_with_unmatched_native = profiles.resolve("tkr-other-worker", None)?;
        assert_eq!(
            context_with_unmatched_native.executor.as_deref(),
            Some("memory")
        );
        assert!(
            !context_with_unmatched_native
                .resources
                .contains_key("native")
        );
        assert_eq!(context_with_unmatched_native.resources["cpu"], 2);
        assert_eq!(
            profiles
                .resolve("tkr-qulacs-worker", Some("gpu"))?
                .resources["gpu"],
            "500m"
        );
        assert!(
            profiles
                .resolve("tkr-qulacs-worker", Some("a100-4node"))
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn rejects_zero_max_concurrency_in_profile_fixture() {
        assert!(ResourceProfiles::load(&[profile_fixtures()], "invalid_concurrency").is_err());
    }

    #[test]
    fn abstract_base_profile_cannot_be_selected() {
        assert!(ResourceProfiles::load(&[profile_fixtures()], "base").is_err());
    }

    #[test]
    fn higher_priority_profile_overlays_context_and_retargets_parent() -> Result<()> {
        let fixtures = profile_fixtures().join("retarget");
        let profiles = ResourceProfiles::load(
            &[fixtures.join("project"), fixtures.join("home")],
            "cluster",
        )?;
        let context = profiles.resolve("tkr-qulacs-worker", None)?;
        assert_eq!(context.executor.as_deref(), Some("memory"));
        assert_eq!(context.resources["cpu"], 8.0);
        assert_eq!(context.resources["account"], "personal");
        assert_eq!(
            profiles
                .resolve("tkr-other-worker", Some("cpu_only"))?
                .resources["cpu"],
            8.0
        );
        Ok(())
    }

    #[test]
    fn a_mapping_cannot_shadow_an_unrelated_implicit_context() {
        let root = profile_fixtures().join("mapping_collision");
        assert!(ResourceProfiles::load(&[root], "cluster").is_err());
    }

    #[test]
    fn rejects_profile_and_context_cycles() {
        let root = profile_fixtures().join("cycles");
        assert!(ResourceProfiles::load(std::slice::from_ref(&root), "profile_cycle").is_err());
        assert!(ResourceProfiles::load(&[root], "context_cycle").is_err());
    }

    #[test]
    fn rejects_invalid_quantities_and_escaping_aliases() {
        let fixtures = profile_fixtures().join("invalid_settings");
        assert!(ResourceProfiles::load(&[fixtures.join("invalid_quantities")], "local").is_err());
        assert!(ResourceProfiles::load(&[fixtures.join("escaping_alias")], "local").is_err());
    }
}
