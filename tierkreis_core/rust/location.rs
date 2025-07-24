use pyo3::{exceptions::PyValueError, prelude::*, types::PyIterator};
use serde::{Deserialize, Serialize};

#[pymodule(submodule)]
pub mod location {
    use std::collections::{HashMap, VecDeque};

    use pyo3::types::{IntoPyDict, PyType};

    use crate::identifiers::identifiers::{ExteriorOrValueRef, NodeIndex};

    use super::*;

    /// Hack: workaround for https://github.com/PyO3/pyo3/issues/759
    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        Python::attach(|py| {
            py.import("sys")?
                .getattr("modules")?
                .set_item("tierkreis_core._tierkreis_core.location", m)
        })
    }

    #[pyclass(eq, frozen, hash)]
    #[derive(Clone, PartialEq, Serialize, Deserialize, Hash, Debug)]
    pub enum NodeStep {
        // The root step, should always be the first step if
        // it is present at all.
        Root {},
        // A single Node, such as a function, cosnt or an eval.
        Node { node_index: NodeIndex },
        Loop { loop_index: usize },
        Map { map_index: usize },
        // A backtraking step to a location with exterior refs
        Exterior {},
    }

    impl std::fmt::Display for NodeStep {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                NodeStep::Root {} => write!(f, "-")?,
                NodeStep::Node { node_index } => write!(f, "N{}", node_index.0)?,
                NodeStep::Loop { loop_index } => write!(f, "L{}", loop_index)?,
                NodeStep::Map { map_index } => write!(f, "M{}", map_index)?,
                NodeStep::Exterior {} => write!(f, "E")?,
            }
            Ok(())
        }
    }

    // These methods are mostly for tests, users should prefer
    // using the methods on Loc instead.
    #[pymethods]
    impl NodeStep {
        #[new]
        pub fn new(steps: &str) -> PyResult<Self> {
            match (steps.get(0..1), steps.get(1..)) {
                (Some("-"), Some("")) => Ok(NodeStep::Root {}),
                (Some("N"), Some(idx_str)) => Ok(NodeStep::Node {
                    node_index: NodeIndex(idx_str.parse()?),
                }),
                (Some("L"), Some(idx_str)) => Ok(NodeStep::Loop {
                    loop_index: idx_str.parse()?,
                }),
                (Some("M"), Some(idx_str)) => Ok(NodeStep::Map {
                    map_index: idx_str.parse()?,
                }),
                (Some("E"), Some("")) => Ok(NodeStep::Exterior {}),
                (step, index) => Err(PyValueError::new_err(format!(
                    "Could not parse Loc: {} with step {:?} and index {:?}",
                    steps, step, index
                ))),
            }
        }

        fn is_root(&self) -> bool {
            self == &NodeStep::Root {}
        }

        pub fn __iter__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyIterator>> {
            match self {
                Self::Root {} => PyIterator::from_object(&"-".into_pyobject(py)?.into_any()),
                Self::Node { node_index } => {
                    PyIterator::from_object(&("N", *node_index).into_pyobject(py)?.into_any())
                }
                Self::Loop { loop_index } => {
                    PyIterator::from_object(&("L", loop_index).into_pyobject(py)?.into_any())
                }
                Self::Map { map_index } => {
                    PyIterator::from_object(&("M", map_index).into_pyobject(py)?.into_any())
                }
                Self::Exterior {} => PyIterator::from_object(&"E".into_pyobject(py)?.into_any()),
            }
        }

        pub fn __repr__(&self) -> String {
            format!("NodeStep('{}')", self)
        }

        pub fn __str__(&self) -> String {
            format!("{}", self)
        }
    }

    /// A location where a Value is stored, expressed as
    /// a sequence of NodeStep.
    #[pyclass(eq, frozen, hash)]
    #[derive(Clone, PartialEq, Serialize, Deserialize, Hash, Debug)]
    pub struct Loc {
        steps: VecDeque<NodeStep>,
    }

    impl std::fmt::Display for Loc {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.steps
                .front()
                .map(|first_step| write!(f, "{}", first_step))
                .transpose()?;
            for step in self.steps.iter().skip(1) {
                write!(f, ".{}", step)?;
            }
            Ok(())
        }
    }

    #[derive(FromPyObject)]
    pub enum NodeIndexOrPositiveInt {
        NodeIndex(NodeIndex),
        Int(usize),
    }

    #[derive(FromPyObject)]
    pub enum LocOrString {
        Loc(Loc),
        String(String),
    }

    #[pymethods]
    impl Loc {
        #[new]
        #[pyo3(signature = (k: "str" = "-"))]
        pub fn new(k: &str) -> PyResult<Self> {
            let parts = k.split_terminator(".");
            let mut steps = VecDeque::new();
            for part in parts {
                steps.push_back(NodeStep::new(part)?);
            }
            Ok(Self { steps })
        }

        pub fn extend_from_ref(&self, value_ref: ExteriorOrValueRef) -> Self {
            let mut steps = self.steps.clone();
            match value_ref {
                ExteriorOrValueRef::Exterior(_) => steps.push_back(NodeStep::Exterior {}),
                ExteriorOrValueRef::Value(value_ref) => steps.push_back(NodeStep::Node {
                    node_index: value_ref.node_index(),
                }),
            };
            Loc { steps }
        }

        pub fn exterior(&self) -> Self {
            let mut steps = self.steps.clone();
            steps.push_back(NodeStep::Exterior {});
            Loc { steps }
        }

        #[pyo3(name = "N")]
        pub fn append_node(&self, node_index: NodeIndexOrPositiveInt) -> Self {
            let mut steps = self.steps.clone();
            let node_index = match node_index {
                NodeIndexOrPositiveInt::NodeIndex(node_index) => node_index,
                NodeIndexOrPositiveInt::Int(i) => NodeIndex(i),
            };
            steps.push_back(NodeStep::Node { node_index });
            Loc { steps }
        }

        #[pyo3(name = "L")]
        pub fn append_loop(&self, loop_index: usize) -> Self {
            let mut steps = self.steps.clone();
            steps.push_back(NodeStep::Loop { loop_index });
            Loc { steps }
        }

        #[pyo3(name = "M")]
        pub fn append_map(&self, map_index: usize) -> Self {
            let mut steps = self.steps.clone();
            steps.push_back(NodeStep::Map { map_index });
            Loc { steps }
        }

        #[staticmethod]
        #[pyo3(signature = (
            steps: "typing.Sequence[NodeStep]",
        ))]
        pub fn from_steps(steps: Vec<NodeStep>) -> Self {
            Self {
                steps: steps.into(),
            }
        }

        #[pyo3(signature = () -> "list[Loc]")]
        pub fn steps(&self) -> Vec<NodeStep> {
            self.steps.iter().cloned().collect()
        }

        /// `parent` will get the last step in Loc, unless the last
        /// step is a `Loop` step, in which case it will get the
        /// previous iteration of the `Loop` step by subtracting 1.
        #[pyo3(signature = () -> "Loc | None")]
        pub fn parent(&self) -> Option<Self> {
            self.steps.back().map(|last_step| match last_step {
                NodeStep::Root {} => Loc {
                    steps: VecDeque::new(),
                },
                NodeStep::Node { .. } | NodeStep::Map { .. } | NodeStep::Exterior { .. } => {
                    let mut steps = self.steps.clone();
                    steps.pop_back();

                    Loc { steps }
                }
                NodeStep::Loop { loop_index } if *loop_index == 0 => {
                    let mut steps = self.steps.clone();
                    steps.pop_back();

                    Loc { steps }
                }
                NodeStep::Loop { loop_index } => {
                    let mut steps = self.steps.clone();
                    steps.pop_back();
                    steps.push_back(NodeStep::Loop {
                        loop_index: loop_index - 1,
                    });

                    Loc { steps }
                }
            })
        }

        // TODO: this needs tests
        pub fn startswith(&self, other: &Loc) -> bool {
            self.steps
                .iter()
                .zip(other.steps.iter())
                .all(|(x, y)| x == y)
        }

        // Returns true if he Loc only contains a root node.
        pub fn is_root_loc(&self) -> bool {
            self.steps.len() == 1 && self.steps.front() == Some(&NodeStep::Root {})
        }

        // Returns true if the last step in the sequence is an exterior step.
        pub fn last_step_exterior(&self) -> bool {
            self.steps.back() == Some(&NodeStep::Exterior {})
        }

        // Returns the index of the step if it is a loop or map step.
        #[pyo3(signature = () -> "int | None")]
        pub fn peek_index(&self) -> Option<usize> {
            self.steps.back().and_then(|step| match step {
                NodeStep::Root {} => None,
                NodeStep::Node { .. } => None,
                NodeStep::Loop { loop_index } => Some(*loop_index),
                NodeStep::Map { map_index } => Some(*map_index),
                NodeStep::Exterior {} => None,
            })
        }

        pub fn partial_locs(&self) -> Vec<Loc> {
            let mut partial_locs = Vec::new();
            let mut intermediate = VecDeque::new();
            for step in &self.steps {
                intermediate.push_back(step.clone());
                partial_locs.push(Loc {
                    steps: intermediate.clone(),
                });
            }
            partial_locs
        }

        #[pyo3(signature = () -> "tuple[NodeStep, Loc]")]
        pub fn pop_first(&self) -> PyResult<(NodeStep, Loc)> {
            if self.steps.len() == 1 && self.steps.front() == Some(&NodeStep::Root {}) {
                return Ok((
                    NodeStep::Root {},
                    Loc {
                        steps: VecDeque::new(),
                    },
                ));
            }
            if self.steps.len() < 2 {
                return Err(PyValueError::new_err("Cannot pop from empty Loc"));
            }
            let mut steps = self.steps.clone();
            // We never pop the root step and instead pop the next one along.
            let first = steps.remove(1);
            if first == Some(NodeStep::Root {}) {
                return Err(PyValueError::new_err("Malformed Loc"));
            }
            Ok((first.unwrap(), Loc { steps }))
        }

        #[pyo3(signature = () -> "tuple[NodeStep, Loc]")]
        pub fn pop_last(&self) -> PyResult<(NodeStep, Loc)> {
            if self.steps.len() == 1 && self.steps.front() == Some(&NodeStep::Root {}) {
                return Ok((
                    NodeStep::Root {},
                    Loc {
                        steps: VecDeque::new(),
                    },
                ));
            }
            if self.steps.len() < 2 {
                return Err(PyValueError::new_err("Cannot pop from empty Loc"));
            }
            let mut steps = self.steps.clone();
            let first = steps.pop_back();
            if first == Some(NodeStep::Root {}) {
                return Err(PyValueError::new_err("Malformed Loc"));
            }
            Ok((first.unwrap(), Loc { steps }))
        }

        pub fn __fspath__(&self) -> String {
            format!("{}", self)
        }

        pub fn __repr__(&self) -> String {
            format!("Loc('{}')", self)
        }

        pub fn __str__(&self) -> String {
            format!("{}", self)
        }

        // pydantic compatibility
        #[classmethod]
        pub fn __get_pydantic_core_schema__(
            cls: &Bound<'_, PyType>,
            _source: &Bound<'_, PyType>,
            _handler: &Bound<'_, PyAny>,
        ) -> PyResult<Py<PyAny>> {
            let pydantic = PyModule::import(cls.py(), "pydantic_core")?;
            let core_schema = pydantic.getattr("core_schema")?;

            let validate_fn = cls.getattr("_validate")?;
            // Used to validate the output of the schema
            let any_schema = core_schema.call_method0("any_schema")?;

            let serialization = core_schema.call_method0("to_string_ser_schema")?;

            let mut validator_kwargs = HashMap::new();
            validator_kwargs.insert("serialization", serialization);

            let schema = core_schema.call_method(
                "no_info_before_validator_function",
                (validate_fn, any_schema),
                Some(&validator_kwargs.into_py_dict(cls.py())?),
            )?;

            Ok(schema.unbind())
        }

        #[staticmethod]
        pub fn _validate(value: LocOrString) -> PyResult<Loc> {
            match value {
                LocOrString::Loc(loc) => Ok(loc),
                LocOrString::String(s) => Loc::new(&s),
            }
        }
    }
}
