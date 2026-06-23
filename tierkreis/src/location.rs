/*!
This module defines the [`Location`] struct that is used throughout the Tierkreis
runtime to specify the place in a Workflow graph that something has happened.
*/
use diesel::backend::Backend;
use diesel::sqlite::Sqlite;
use miette::{IntoDiagnostic, miette};
use portgraph::NodeIndex;
use std::str::FromStr;

use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::Text;
use diesel::{AsExpression, FromSqlRow};

/// A component of the path for a [`Location`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum LocationComponent {
    /// The [`NodeIndex`] of a Node in a Graph or Subgraph inside a higher order node.
    Node {
        /// The location of the node within the graph.
        node: NodeIndex,
    },
    /// The [`LoopIndex`] of a Loop node, independent from the [`NodeIndex`] of the Loop node.
    LoopIndex {
        /// The index of the "virtual" loop node within in the graph.
        index: u32,
    },
    /// The [`MapIndex`] of a Map node, independent from the [`NodeIndex`] of the Map node.
    MapIndex {
        /// The index of the "virtual" map element within in the graph.
        index: u32,
    },
}

impl LocationComponent {
    /// Construct a new [`LocationComponent`] from a &str.
    ///
    /// # Errors
    ///
    /// Will return Err if the &str is malformed and cannot be parsed.
    pub fn new(step: &str) -> miette::Result<Self> {
        match (step.get(0..1), step.get(1..)) {
            (Some("N"), Some(idx_str)) => Ok(LocationComponent::Node {
                node: NodeIndex::new(idx_str.parse().into_diagnostic()?),
            }),
            (Some("L"), Some(idx_str)) => Ok(LocationComponent::LoopIndex {
                index: idx_str.parse().into_diagnostic()?,
            }),
            (Some("M"), Some(idx_str)) => Ok(LocationComponent::MapIndex {
                index: idx_str.parse().into_diagnostic()?,
            }),
            (tag, index) => Err(miette!(
                "Could not parse Loc: {} with tag {:?} and index {:?}",
                step,
                tag,
                index
            )),
        }
    }
}

impl std::fmt::Display for LocationComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocationComponent::Node { node } => write!(f, "N{}", node.index())?,
            LocationComponent::LoopIndex { index } => write!(f, "L{index}")?,
            LocationComponent::MapIndex { index } => write!(f, "M{index}")?,
        }
        Ok(())
    }
}

/// A [`Location`] struct describes where a computation is happening in a higher
/// order tierkreis graph.
///
/// A [`Location`] consists of a path of components that point to either:
///
/// * The "root" of the Graph itself (if the path is empty).
/// * A specific node in a Graph or a Subgraph inside a higher order node.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, AsExpression, FromSqlRow)]
#[diesel(sql_type = Text)]
pub struct Location(Vec<LocationComponent>);

impl Location {
    /// Construct a new [`Location`] from a &str.
    ///
    /// # Errors
    ///
    /// Will return Err if the &str is malformed and cannot be parsed.
    pub fn new(k: &str) -> miette::Result<Self> {
        let parts = k.split_terminator('.');
        let mut steps = Vec::new();
        for part in parts {
            steps.push(LocationComponent::new(part)?);
        }
        Ok(Self(steps))
    }

    /// Construct a [`Location`] that represents the "root" Location.
    #[must_use]
    pub fn root() -> Self {
        Self(Vec::new())
    }

    /// Construct a [`Location`] from an iterator of [`usize`].
    ///
    /// This method cannot be used to construct Map or Loop components.
    pub fn from_usize_iter(nodes: impl IntoIterator<Item = usize>) -> Self {
        Self(
            nodes
                .into_iter()
                .map(NodeIndex::new)
                .map(|node_index| LocationComponent::Node { node: node_index })
                .collect(),
        )
    }

    /// Returns true if the Location represents the "root" Location.
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    /// Construct a [`Location`] from an iterator of [`NodeIndex`].
    pub fn from_node_index_iter(nodes: impl IntoIterator<Item = NodeIndex>) -> Self {
        Self(
            nodes
                .into_iter()
                .map(|node_index| LocationComponent::Node { node: node_index })
                .collect(),
        )
    }

    /// Extend the [`Location`] struct with a Node component with the specified [`NodeIndex`].
    #[must_use]
    pub fn with_node(&self, node: NodeIndex) -> Location {
        let mut inner = self.0.clone();
        inner.push(LocationComponent::Node { node });
        Location(inner)
    }

    /// Extend the [`Location`] struct with a Node component with the specified [`NodeIndex`].
    #[must_use]
    pub fn with_loop_index(&self, index: u32) -> Location {
        let mut inner = self.0.clone();
        inner.push(LocationComponent::LoopIndex { index });
        Location(inner)
    }

    /// Extend the [`Location`] struct with a Node component with the specified [`NodeIndex`].
    ///
    /// # Panics
    ///
    /// Will panic if the index cannot be converted into a `u32`.
    #[must_use]
    pub fn with_map_index(&self, index: usize) -> Location {
        let mut inner = self.0.clone();
        inner.push(LocationComponent::MapIndex {
            index: u32::try_from(index).expect("Map index > U32_MAX"),
        });
        Location(inner)
    }

    /// Obtain the "Parent" Location.
    #[must_use]
    pub fn parent(&self) -> Location {
        let mut components = self.0.clone();
        components.pop();
        Location(components)
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0
            .first()
            .map(|first_step| write!(f, "{first_step}"))
            .transpose()?;
        for step in self.0.iter().skip(1) {
            write!(f, ".{step}")?;
        }
        Ok(())
    }
}

impl FromStr for Location {
    type Err = miette::ErrReport;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl ToSql<Text, Sqlite> for Location {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

impl<DB: Backend> FromSql<Text, DB> for Location
where
    String: FromSql<Text, DB>,
{
    fn from_sql(value: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let serialized = <String as FromSql<Text, DB>>::from_sql(value)?;
        serialized.parse::<Location>().map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_location_serialization() -> miette::Result<()> {
        let location = Location::from_usize_iter([2, 4, 8]);
        let serialized = location.to_string();
        let parsed = serialized.parse::<Location>()?;
        assert_eq!(location, parsed);

        Ok(())
    }

    #[test]
    fn roundtrip_location_serialization_from_str() -> miette::Result<()> {
        let location = Location::new("N2.N4.M5.N5.L6")?;
        let serialized = location.to_string();
        let parsed = serialized.parse::<Location>()?;
        assert_eq!(location, parsed);

        Ok(())
    }

    #[test]
    fn root_serializes_as_empty_path() -> miette::Result<()> {
        let root = Location::root();
        assert_eq!(root.to_string(), "");
        assert_eq!("".parse::<Location>()?, root);

        Ok(())
    }
}
