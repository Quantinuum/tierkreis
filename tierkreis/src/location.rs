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

    /// Iterate over the [`LocationComponent`]s that make up this [`Location`].
    pub fn components(&self) -> impl Iterator<Item = &LocationComponent> {
        self.0.iter()
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

/// A component of a [`LocationPattern`], either matching a specific [`LocationComponent`]
/// or acting as a wildcard for any Loop or Map iteration index.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PatternComponent {
    /// Matches only the given [`LocationComponent`] exactly.
    Exact(LocationComponent),
    /// Matches any `LoopIndex` value, e.g. "L*".
    AnyLoopIndex,
    /// Matches any `MapIndex` value, e.g. "M*".
    AnyMapIndex,
}

impl std::fmt::Display for PatternComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatternComponent::Exact(component) => write!(f, "{component}"),
            PatternComponent::AnyLoopIndex => write!(f, "L*"),
            PatternComponent::AnyMapIndex => write!(f, "M*"),
        }
    }
}

impl PatternComponent {
    fn new(step: &str) -> miette::Result<Self> {
        match step {
            "L*" => Ok(PatternComponent::AnyLoopIndex),
            "M*" => Ok(PatternComponent::AnyMapIndex),
            other => Ok(PatternComponent::Exact(LocationComponent::new(other)?)),
        }
    }
}

/// A [`LocationPattern`] describes a set of [`Location`]s that share a fixed structure.
///
/// For example the pattern `"N3.L*.N1"` matches the `Location`s of node `N1` inside
/// the subgraph of loop node `N3`, for every iteration of that loop.
/// The pattern `"N3.L*"` matches the output node of every iteration of the loop node `N3`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LocationPattern(Vec<PatternComponent>);

impl LocationPattern {
    /// Construct a new [`LocationPattern`] from a &str.
    ///
    /// # Errors
    ///
    /// Will return Err if the &str is malformed and cannot be parsed.
    pub fn new(k: &str) -> miette::Result<Self> {
        let parts = k.split_terminator('.');
        let mut steps = Vec::new();
        for part in parts {
            steps.push(PatternComponent::new(part)?);
        }
        Ok(Self(steps))
    }

    /// Returns true if the given [`Location`] matches this pattern.
    #[must_use]
    pub fn matches(&self, location: &Location) -> bool {
        if self.0.len() != location.0.len() {
            return false;
        }
        self.0
            .iter()
            .zip(location.0.iter())
            .all(|(pattern, component)| match (pattern, component) {
                (PatternComponent::Exact(expected), actual) => expected == actual,
                (PatternComponent::AnyLoopIndex, LocationComponent::LoopIndex { .. })
                | (PatternComponent::AnyMapIndex, LocationComponent::MapIndex { .. }) => true,
                _ => false,
            })
    }

    /// Returns the wildcard indices (in pattern order) of a matching [`Location`].
    ///
    /// Useful for sorting matches by iteration order. Only meaningful if
    /// `self.matches(location)` is `true`; otherwise the result is unspecified.
    /// E.g. for the pattern `"N3.L*.N1.L*"`, the sort key for the location `"N3.L2.N1.L0.N3"` would be `[2, 0]`.
    #[must_use]
    pub fn sort_key(&self, location: &Location) -> Vec<u32> {
        self.0
            .iter()
            .zip(&location.0)
            .filter_map(|(pattern, component)| match (pattern, component) {
                (PatternComponent::AnyLoopIndex, LocationComponent::LoopIndex { index })
                | (PatternComponent::AnyMapIndex, LocationComponent::MapIndex { index }) => {
                    Some(*index)
                }
                _ => None,
            })
            .collect()
    }

    /// Returns a SQL `LIKE` pattern that is guaranteed to match a superset of the
    /// [`Location`]s that satisfy this [`LocationPattern`].
    ///
    /// Can be used with `LIKE ... ESCAPE '\'` as a cheap, index-friendly prefilter. 
    /// Callers must still apply [`LocationPattern::matches`] on the results.
    #[must_use]
    pub fn as_sql_like_pattern(&self) -> String {
        self.0
            .iter()
            .map(|component| match component {
                PatternComponent::Exact(component) => component
                    .to_string()
                    .replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_"),
                PatternComponent::AnyLoopIndex | PatternComponent::AnyMapIndex => "%".to_string(),
            })
            .collect::<Vec<_>>()
            .join(".")
    }

    /// Iterate over the [`PatternComponent`]s that make up this [`LocationPattern`].
    pub fn components(&self) -> impl Iterator<Item = &PatternComponent> {
        self.0.iter()
    }

    /// Extend the [`LocationPattern`] with an exact Node component with the specified [`NodeIndex`].
    #[must_use]
    pub fn with_node(&self, node: NodeIndex) -> LocationPattern {
        let mut inner = self.0.clone();
        inner.push(PatternComponent::Exact(LocationComponent::Node { node }));
        LocationPattern(inner)
    }
}

impl std::fmt::Display for LocationPattern {
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

impl FromStr for LocationPattern {
    type Err = miette::ErrReport;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
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

    #[test]
    fn pattern_matches_any_loop_iteration() -> miette::Result<()> {
        let pattern = LocationPattern::new("N3.L*.N1")?;
        assert!(pattern.matches(&Location::new("N3.L0.N1")?));
        assert!(pattern.matches(&Location::new("N3.L7.N1")?));
        assert!(!pattern.matches(&Location::new("N3.L0.N2")?));
        assert!(!pattern.matches(&Location::new("N3.M0.N1")?));
        assert!(!pattern.matches(&Location::new("N3.L0.L0.N1")?));

        let pattern = LocationPattern::new("N3.L*")?;
        assert!(!pattern.matches(&Location::new("N3.L0.N1")?));

        Ok(())
    }

    #[test]
    fn pattern_sort_key_orders_by_iteration() -> miette::Result<()> {
        let pattern = LocationPattern::new("N3.L*.N1")?;
        assert_eq!(pattern.sort_key(&Location::new("N3.L2.N1")?), vec![2]);
        let pattern = LocationPattern::new("N3.L*.N1.L*")?;
        assert_eq!(pattern.sort_key(&Location::new("N3.L2.N1.L0.N3")?), vec![2, 0]);

        Ok(())
    }

    #[test]
    fn pattern_sql_like_escapes_special_characters() -> miette::Result<()> {
        let pattern = LocationPattern::new("N3.L*.N1")?;
        assert_eq!(pattern.as_sql_like_pattern(), "N3.%.N1");

        Ok(())
    }

    #[test]
    fn roundtrip_pattern_serialization() -> miette::Result<()> {
        let pattern = LocationPattern::new("N3.L*.M*.N1")?;
        let serialized = pattern.to_string();
        assert_eq!(serialized, "N3.L*.M*.N1");
        let parsed = serialized.parse::<LocationPattern>()?;
        assert_eq!(pattern, parsed);

        Ok(())
    }
}
