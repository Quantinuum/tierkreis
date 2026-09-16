use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

use miette::{Context, IntoDiagnostic, bail, miette};

use super::templates;

struct WorkerName {
    module: String,
    distribution: String,
}

impl WorkerName {
    fn parse(value: &str) -> miette::Result<Self> {
        let mut chars = value.chars();
        if !matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
            || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            bail!(
                "invalid worker name `{value}`; use ASCII letters, digits, `_`, or `-`, starting with a letter"
            );
        }
        Ok(Self {
            module: value.replace('-', "_").to_ascii_lowercase(),
            distribution: value.replace('_', "-").to_ascii_lowercase(),
        })
    }
}

fn write(path: &Path, contents: &str) -> miette::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, contents)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to write {}", path.display()))
}

fn render(template: &str, name: &WorkerName) -> String {
    templates::render(template, &name.module, &name.distribution)
}

fn write_worker(root: &Path, name: &WorkerName, external: bool) -> miette::Result<()> {
    write(
        &root.join("README.md"),
        &render(templates::WORKER_README, name),
    )?;
    write(
        &root.join("api/README.md"),
        &render(templates::API_README, name),
    )?;
    write(
        &root.join("api/pyproject.toml"),
        &render(templates::API_PYPROJECT, name),
    )?;
    write(&root.join("api/__init__.py"), "from .api import *\n")?;
    let api_template = if external {
        templates::EXTERNAL_API_PLACEHOLDER
    } else {
        templates::API_STUB
    };
    write(&root.join("api/api.py"), &render(api_template, name))?;
    write(&root.join("__init__.py"), "from .api.api import *\n")?;

    if external {
        return write(
            &root.join("schema").join(format!("{}.tsp", name.module)),
            &render(templates::EXTERNAL_IDL, name),
        );
    }

    write(
        &root.join("pyproject.toml"),
        &render(templates::WORKER_PYPROJECT, name),
    )?;
    let implementation = root.join(format!("tkr_{}_impl", name.module));
    write(
        &implementation.join("main.py"),
        &render(templates::WORKER_MAIN, name),
    )?;
    write(
        &implementation.join("impl.py"),
        &render(templates::WORKER_IMPL, name),
    )?;
    write(
        &implementation.join("__init__.py"),
        "from .impl import worker\n\n__all__ = [\"worker\"]\n",
    )
}

fn create_worker(name: &WorkerName, workers: &Path, external: bool) -> miette::Result<PathBuf> {
    fs::create_dir_all(workers)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to create {}", workers.display()))?;
    let destination = workers.join(&name.module);
    if destination.exists() {
        bail!(
            "worker destination already exists: {}",
            destination.display()
        );
    }

    let staging = tempfile::Builder::new()
        .prefix(".tkr-worker-")
        .tempdir_in(workers)
        .into_diagnostic()
        .wrap_err("failed to create worker staging directory")?;
    write_worker(staging.path(), name, external)?;
    let staging_path = staging.keep();
    if let Err(error) = fs::rename(&staging_path, &destination) {
        let _ = fs::remove_dir_all(&staging_path);
        return Err(error)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to install worker at {}", destination.display()));
    }
    Ok(destination)
}

pub(super) fn init_project(directory: &Path) -> miette::Result<()> {
    let directory = if directory.is_absolute() {
        directory.to_path_buf()
    } else {
        std::env::current_dir()
            .into_diagnostic()
            .wrap_err("failed to resolve current directory")?
            .join(directory)
    };
    fs::create_dir_all(&directory)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to create project directory {}", directory.display()))?;

    let generated = [
        directory.join("tkr"),
        directory.join("workflow_inputs.json"),
        directory.join(crate::config::CONFIG_FILE_NAME),
    ];
    let project_manifest = directory.join("pyproject.toml");
    let create_project_manifest = !project_manifest.exists();
    if let Some(existing) = generated.iter().find(|path| path.exists()) {
        bail!(
            "refusing to overwrite existing generated path: {}",
            existing.display()
        );
    }

    let tree_staging = tempfile::Builder::new()
        .prefix(".tkr-project-")
        .tempdir_in(&directory)
        .into_diagnostic()
        .wrap_err("failed to create project staging directory")?;
    let tree = tree_staging.path().join("tkr");
    fs::create_dir_all(tree.join("graphs")).into_diagnostic()?;
    write(&tree.join("__init__.py"), "")?;
    write(&tree.join("graphs/__init__.py"), "")?;
    write(&tree.join("workers/__init__.py"), "")?;
    let example = WorkerName::parse("example_worker")?;
    write_worker(&tree.join("workers/example_worker"), &example, false)?;
    write(
        &tree.join("graphs/main.py"),
        &render(templates::DEFAULT_GRAPH, &example),
    )?;

    fs::rename(&tree, &generated[0])
        .into_diagnostic()
        .wrap_err("failed to install generated project tree")?;
    if let Err(error) = write(&generated[1], templates::INPUTS)
        .and_then(|()| {
            let config = crate::runtime::RuntimeConfig::for_project(&directory).to_toml_string()?;
            write(&generated[2], &config)
        })
        .and_then(|()| {
            if create_project_manifest {
                write(&project_manifest, templates::PROJECT_PYPROJECT)?;
            }
            Ok(())
        })
        .and_then(|()| {
            fs::create_dir_all(directory.join(".tierkreis/assets"))
                .into_diagnostic()
                .wrap_err("failed to create project runtime directory")
        })
    {
        let _ = fs::remove_dir_all(&generated[0]);
        let _ = fs::remove_dir_all(directory.join(".tierkreis"));
        let _ = fs::remove_file(&generated[1]);
        let _ = fs::remove_file(&generated[2]);
        if create_project_manifest {
            let _ = fs::remove_file(&project_manifest);
        }
        return Err(error);
    }

    println!("Created Tierkreis project in {}", directory.display());
    if create_project_manifest {
        println!("Created pyproject.toml with Tierkreis and worker workspace dependencies.");
    } else {
        println!("Kept the existing pyproject.toml; ensure it depends on `tierkreis`.");
    }
    println!("Next: run `uv sync`, then inspect `tkr/graphs/main.py`.");
    Ok(())
}

pub(super) fn init_worker(
    value: &str,
    directory: Option<&Path>,
    external: bool,
) -> miette::Result<()> {
    let name = WorkerName::parse(value)?;
    let workers = match directory {
        Some(path) => path.to_path_buf(),
        None => discover_project_root()?.join("tkr/workers"),
    };
    let destination = create_worker(&name, &workers, external)?;
    println!("Created worker in {}", destination.display());
    Ok(())
}

fn discover_project_root() -> miette::Result<PathBuf> {
    let current = std::env::current_dir()
        .into_diagnostic()
        .wrap_err("failed to resolve current directory")?;
    current
        .ancestors()
        .find(|path| path.join(crate::config::CONFIG_FILE_NAME).is_file())
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            miette!(
                "no Tierkreis project found; run `tkr init project` or pass an explicit directory"
            )
        })
}

fn find_idl(worker: &Path) -> miette::Result<Option<PathBuf>> {
    let mut candidates = Vec::new();
    for entry in fs::read_dir(worker)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to inspect {}", worker.display()))?
    {
        let path = entry.into_diagnostic()?.path();
        if path.is_dir() {
            for nested in fs::read_dir(&path).into_diagnostic()? {
                let candidate = nested.into_diagnostic()?.path();
                if candidate.extension() == Some(OsStr::new("tsp")) {
                    candidates.push(candidate);
                }
            }
        }
    }
    candidates.sort();
    match candidates.as_slice() {
        [] => Ok(None),
        [candidate] => Ok(Some(candidate.clone())),
        _ => bail!(
            "worker {} contains multiple TypeSpec files; keep one schema entry point",
            worker.display()
        ),
    }
}

fn ensure_success(status: ExitStatus, action: &str) -> miette::Result<()> {
    if status.success() {
        Ok(())
    } else {
        bail!("{action} failed with {status}")
    }
}

pub(super) fn generate_stubs(
    workers_directory: Option<&Path>,
    output: &Path,
) -> miette::Result<()> {
    if output.is_absolute() {
        bail!("stub output must be relative to each worker directory");
    }
    let workers = match workers_directory {
        Some(path) => path.to_path_buf(),
        None => discover_project_root()?.join("tkr/workers"),
    };
    if !workers.is_dir() {
        bail!("worker directory does not exist: {}", workers.display());
    }
    let uv = which::which("uv")
        .into_diagnostic()
        .wrap_err("`uv` is required to generate worker stubs; install uv and retry")?;
    let mut worker_paths = fs::read_dir(&workers)
        .into_diagnostic()?
        .map(|entry| entry.map(|value| value.path()).into_diagnostic())
        .collect::<miette::Result<Vec<_>>>()?;
    worker_paths
        .retain(|path| path.is_dir() && path.file_name().is_none_or(|n| n != "__pycache__"));
    worker_paths.sort();

    for worker in worker_paths {
        let worker_name = worker
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| miette!("worker path is not valid UTF-8: {}", worker.display()))?;
        let destination = worker.join(output);
        if let Some(idl) = find_idl(&worker)? {
            let status = Command::new(&uv)
                .args([
                    "run",
                    "--active",
                    "--no-project",
                    "python",
                    "-m",
                    "tierkreis.cli.bridge",
                ])
                .arg("stubs-from-idl")
                .arg(&idl)
                .arg(&destination)
                .status()
                .into_diagnostic()
                .wrap_err_with(|| format!("failed to start stub generator for {worker_name}"))?;
            ensure_success(status, &format!("stub generation for {worker_name}"))?;
        } else {
            let main = format!("tkr_{worker_name}_impl/main.py");
            let status = Command::new(&uv)
                .args(["run", "--active"])
                .arg(main)
                .arg("--stubs-path")
                .arg(output)
                .current_dir(&worker)
                .status()
                .into_diagnostic()
                .wrap_err_with(|| format!("failed to start worker {worker_name}"))?;
            ensure_success(status, &format!("stub generation for {worker_name}"))?;
        }
        println!("Generated {}", destination.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_names_are_normalized() {
        let name = WorkerName::parse("My-worker_2").unwrap();
        assert_eq!(name.module, "my_worker_2");
        assert_eq!(name.distribution, "my-worker-2");
        assert!(WorkerName::parse("2worker").is_err());
        assert!(WorkerName::parse("bad worker").is_err());
    }

    #[test]
    fn project_creation_is_safe_and_complete() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path()).unwrap();
        assert!(temp.path().join("tierkreis.toml").is_file());
        assert!(temp.path().join("pyproject.toml").is_file());
        assert!(temp.path().join(".tierkreis/assets").is_dir());
        assert!(temp.path().join("workflow_inputs.json").is_file());
        assert!(
            temp.path()
                .join("tkr/workers/example_worker/api/api.py")
                .is_file()
        );
        let api =
            fs::read_to_string(temp.path().join("tkr/workers/example_worker/api/api.py")).unwrap();
        assert!(api.starts_with("\"\"\""));
        assert!(temp.path().join("tkr/graphs/main.py").is_file());
        assert!(init_project(temp.path()).is_err());
    }

    #[test]
    fn external_worker_contains_schema_but_no_python_implementation() {
        let temp = tempfile::tempdir().unwrap();
        let name = WorkerName::parse("external-worker").unwrap();
        let destination = create_worker(&name, temp.path(), true).unwrap();
        assert!(destination.join("schema/external_worker.tsp").is_file());
        assert!(!destination.join("pyproject.toml").exists());
        assert!(destination.join("api/api.py").is_file());
    }
}
