use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    fs::File,
    io::Read,
    path::Path,
};

use miette::{
    Error, IntoDiagnostic, LabeledSpan, MietteDiagnostic, NamedSource, SourceOffset, SourceSpan,
    miette,
};
use pyo3::{exceptions::PySyntaxError, prelude::*, types::IntoPyDict};

/// Utility macro is make nicer diagnostics and return early when handling python exceptions.
macro_rules! getattr_or_early_return {
    ($obj:ident, $attr:expr) => {{
        let attr_any = match $obj.getattr($attr).into_diagnostic() {
            Ok(attr) => attr,
            Err(err) => return err,
        };

        let attr = match attr_any.extract().into_diagnostic() {
            Ok(attr) => attr,
            Err(err) => return err,
        };

        attr
    }};
}

pub fn run(path: &Path) -> miette::Result<()> {
    Python::attach(|py| {
        let path = path.canonicalize().into_diagnostic()?;

        let mut file = File::open(&path).into_diagnostic()?;
        let mut code_buf = String::new();
        file.read_to_string(&mut code_buf).into_diagnostic()?;
        code_buf.push('\x00');

        let code = CStr::from_bytes_until_nul(code_buf.as_bytes()).into_diagnostic()?;

        let file_name = path.file_name().ok_or(miette!("no file name"))?;
        let file_name_str = file_name
            .to_str()
            .ok_or(miette!("failed to convert to cstring"))?;
        let file_name_cstr = CString::new(file_name_str).into_diagnostic()?;

        let module = path.file_stem().ok_or(miette!("no file stem"))?;
        let module_name_cstr = CString::new(
            module
                .to_str()
                .ok_or(miette!("failed to convert to cstring"))?,
        )
        .into_diagnostic()?;

        let module = PyModule::from_code(py, &code, &file_name_cstr, &module_name_cstr).map_err(
            |err: PyErr| {
                if err.is_instance_of::<PySyntaxError>(py) {
                    let err_value = err.value(py);
                    let message: String = getattr_or_early_return!(err_value, "msg");
                    let lineno: usize = getattr_or_early_return!(err_value, "lineno");
                    let offset: usize = getattr_or_early_return!(err_value, "offset");
                    let end_offset: usize = getattr_or_early_return!(err_value, "end_offset");
                    let filename: String = getattr_or_early_return!(err_value, "filename");

                    let labels = vec![LabeledSpan::new_primary_with_span(
                        Some(message.clone()),
                        SourceSpan::new(
                            SourceOffset::from_location(&code_buf, lineno, offset),
                            end_offset - offset,
                        ),
                    )];
                    let diagnostic = MietteDiagnostic::new(message).and_labels(labels).with_help(
                        "Tierkreis requires a valid python module to construct a Workflow",
                    );

                    let source_code =
                        NamedSource::new(filename, code_buf.clone()).with_language("Python");

                    return Error::new(diagnostic).with_source_code(source_code);
                };

                miette!("Failed to load python module: {}", err.to_string())
            },
        )?;

        let workflow = module
            .getattr("workflow")
            .into_diagnostic()
            .map_err(|err| {
                let diagnostic = MietteDiagnostic::new("No 'workflow' attribute found in module")
                    .with_help("Tierkreis requires an attribute called 'workflow'");
                let rich_error = Error::new(diagnostic).with_source_code(
                    NamedSource::new(file_name_str, code_buf.clone()).with_language("Python"),
                );
                rich_error.wrap_err(err)
            })?;

        let tierkreis_cli = PyModule::import(py, "tierkreis.cli.run_workflow").into_diagnostic()?;
        let run_workflow = tierkreis_cli.getattr("run_workflow").into_diagnostic()?;

        let inputs = HashMap::<String, String>::new();
        let mut kwargs = HashMap::new();
        kwargs.insert("print_output", true);
        let kwargs = kwargs.into_py_dict(py).into_diagnostic()?;

        run_workflow
            .call((workflow, inputs), Some(&kwargs))
            .into_diagnostic()
            .map_err(|err| {
                err.with_source_code(
                    NamedSource::new(file_name_str, code_buf).with_language("Python"),
                )
            })?;

        Ok(())
    })
}
