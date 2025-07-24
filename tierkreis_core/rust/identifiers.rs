use pyo3::{prelude::*, types::PyIterator};
use serde::{Deserialize, Serialize};

#[pymodule(submodule)]
pub mod identifiers {
    use super::*;

    /// Hack: workaround for https://github.com/PyO3/pyo3/issues/759
    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        Python::attach(|py| {
            py.import("sys")?
                .getattr("modules")?
                .set_item("tierkreis_core._tierkreis_core.identifiers", m)
        })
    }

    pub type PortID = String;

    #[pyclass(eq, frozen, hash)]
    #[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Hash, Debug)]
    pub struct NodeIndex(pub usize);

    impl std::fmt::Display for NodeIndex {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "NodeIndex({})", self.0)?;
            Ok(())
        }
    }

    #[pymethods]
    impl NodeIndex {
        #[new]
        pub fn new(val: usize) -> Self {
            Self(val)
        }

        pub fn __repr__(&self) -> String {
            format!("{}", self)
        }

        pub fn __str__(&self) -> String {
            self.0.to_string()
        }

        pub fn __int__(&self) -> PyResult<i32> {
            let x = self.0.try_into()?;
            Ok(x)
        }
    }

    /// A reference to a value on an edge of a graph.
    #[pyclass(eq, frozen, generic)]
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct ValueRef(pub usize, pub String);

    impl std::fmt::Display for ValueRef {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "ValueRef({}, '{}')", self.0, self.1)?;
            Ok(())
        }
    }

    #[pymethods]
    impl ValueRef {
        #[new]
        pub fn new(node_index: NodeIndex, port_id: PortID) -> Self {
            Self(node_index.0, port_id)
        }

        #[getter]
        pub fn node_index(&self) -> NodeIndex {
            NodeIndex(self.0)
        }

        #[getter]
        pub fn port_id(&self) -> PortID {
            self.1.clone()
        }

        pub fn __iter__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyIterator>> {
            PyIterator::from_object(
                &(NodeIndex(self.0), self.1.clone())
                    .into_pyobject(py)?
                    .into_any(),
            )
        }

        pub fn __repr__(&self) -> String {
            format!("{}", self)
        }

        pub fn __str__(&self) -> String {
            format!("({}, '{}')", self.0, self.1)
        }
    }

    /// A reference from outside the current scope of execution.
    ///
    /// This could be an input or the body of a node.
    #[pyclass(eq, frozen)]
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct ExteriorRef(pub PortID);

    impl std::fmt::Display for ExteriorRef {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "ExteriorRef({})", self.0)?;
            Ok(())
        }
    }

    #[pymethods]
    impl ExteriorRef {
        #[new]
        pub fn new(port_id: PortID) -> Self {
            Self(port_id)
        }

        #[getter]
        pub fn port_id(&self) -> PortID {
            self.0.clone()
        }

        pub fn __iter__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyIterator>> {
            PyIterator::from_object(&(ExteriorRef(self.0.clone())).into_pyobject(py)?.into_any())
        }

        pub fn __repr__(&self) -> String {
            format!("{}", self)
        }

        pub fn __str__(&self) -> String {
            self.0.to_string()
        }
    }

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromPyObject, IntoPyObject)]
    pub enum ExteriorOrValueRef {
        Exterior(ExteriorRef),
        Value(ValueRef),
    }

    impl std::fmt::Display for ExteriorOrValueRef {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Exterior(x) => write!(f, "{}", x),
                Self::Value(x) => write!(f, "{}", x),
            }
        }
    }
}
