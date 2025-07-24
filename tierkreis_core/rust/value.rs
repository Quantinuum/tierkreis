use std::collections::HashMap;

use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

#[pymodule(submodule)]
pub mod value {
    use crate::graph::graph::GraphData;

    use super::*;

    /// Hack: workaround for https://github.com/PyO3/pyo3/issues/759
    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        Python::attach(|py| {
            py.import("sys")?
                .getattr("modules")?
                .set_item("tierkreis_core._tierkreis_core.value", m)
        })
    }

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromPyObject, IntoPyObject)]
    pub enum Value {
        Bool(bool),
        Int(i64),
        Float(f64),
        Str(String),
        List(Vec<Value>),
        Map(HashMap<String, Value>),
        Bytes(Vec<u8>),
        Graph(GraphData),
    }
}
