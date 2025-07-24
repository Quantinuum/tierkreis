pub mod graph;
pub mod identifiers;
pub mod location;
pub mod value;

use pyo3::prelude::*;

#[pymodule]
pub mod _tierkreis_core {
    use super::*;

    /// Hack: workaround for https://github.com/PyO3/pyo3/issues/759
    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        Python::attach(|py| {
            py.import("sys")?
                .getattr("modules")?
                .set_item("tierkreis_core._tierkreis_core", m)
        })
    }

    #[pymodule_export]
    use super::graph::graph;
    #[pymodule_export]
    use super::identifiers::identifiers;
    #[pymodule_export]
    use super::location::location;
    #[pymodule_export]
    use super::value::value;
}
