/*!
This module defines utilities for tierkreis workers.
*/
use std::{
    collections::HashMap,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use miette::{IntoDiagnostic, miette};
use serde::{Deserialize, Serialize};

use crate::builtins;

// Json format expected for Tierkreis workers.
#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct WorkerCallArgs {
    pub(crate) function_name: String,
    pub(crate) inputs: HashMap<String, PathBuf>,
    pub(crate) outputs: HashMap<String, PathBuf>,
    pub(crate) output_dir: PathBuf,
    pub(crate) done_path: PathBuf,
    pub(crate) error_path: PathBuf,
    pub(crate) logs_path: Option<PathBuf>,
}

impl WorkerCallArgs {
    pub(crate) fn try_open(path: &Path) -> miette::Result<Self> {
        let file = std::fs::File::open(path).into_diagnostic()?;
        serde_json::from_reader(file).into_diagnostic()
    }

    pub(crate) fn read_inputs(&self) -> miette::Result<HashMap<String, Vec<u8>>> {
        let mut inputs = HashMap::new();
        for (name, path) in &self.inputs {
            let mut file = std::fs::File::open(path).into_diagnostic()?;

            let mut input_bytes = Vec::new();
            file.read_to_end(&mut input_bytes).into_diagnostic()?;
            inputs.insert(name.clone(), input_bytes);
        }

        Ok(inputs)
    }

    pub(crate) fn write_outputs(&self, outputs: &HashMap<String, Vec<u8>>) -> miette::Result<()> {
        for (name, output_bytes) in outputs {
            let path = self
                .outputs
                .get(name)
                .ok_or_else(|| miette!("No output path found for output: {name}"))?;

            let mut file = std::fs::File::create(path).into_diagnostic()?;
            file.write_all(output_bytes).into_diagnostic()?;
        }
        Ok(())
    }
}

/// Run a worker call for the builtins worker.
///
/// Hidden from documentation as it is only exported for the
/// tkr-builtins binary.
#[doc(hidden)]
pub fn run_builtin_worker_call(path: &Path) -> miette::Result<()> {
    let worker_call_args = WorkerCallArgs::try_open(path)?;

    let outputs = builtins::run_builtin(
        &worker_call_args.function_name,
        &worker_call_args.read_inputs()?,
    )?;

    worker_call_args.write_outputs(&outputs)?;

    Ok(())
}
