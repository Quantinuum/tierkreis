/*!
This module defines the [`Location`] struct that is used throughout the Tierkreis
runtime to specify the place in a Workflow graph that something has happened.
*/
use portgraph::NodeIndex;
use std::str::FromStr;

/// A component of the path for a [`Location`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum LocationComponent {
    /// The [`NodeIndex`] of a Node in a Graph or Subgraph inside a higher order node.
    Node(NodeIndex),
}

/// A [`Location`] struct describes where a computation is happening in a higher
/// order tierkreis graph.
///
/// A [`Location`] consists of a path of components that point to either:
///
/// * The "root" of the Graph itself (if the path is empty).
/// * A specific node in a Graph or a Subgraph inside a higher order node.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Location(Vec<LocationComponent>);

impl Location {
    /// Construct a [`Location`] that represents the "root" Location.
    #[must_use]
    pub fn root() -> Self {
        Self(Vec::new())
    }

    /// Construct a [`Location`] from an iterator of [`usize`].
    pub fn from_usize_iter(nodes: impl IntoIterator<Item = usize>) -> Self {
        Self(
            nodes
                .into_iter()
                .map(NodeIndex::new)
                .map(LocationComponent::Node)
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
        Self(nodes.into_iter().map(LocationComponent::Node).collect())
    }

    /// Extend the [`Location`] struct with a Node component with the specified [`NodeIndex`].
    #[must_use]
    pub fn with_node(&self, node: NodeIndex) -> Location {
        let mut inner = self.0.clone();
        inner.push(LocationComponent::Node(node));
        Location(inner)
    }

    /// Obtain the "Parent" Location.
    #[must_use]
    pub fn parent(&self) -> Location {
        let mut components = self.0.clone();
        components.pop();
        Location(components)
    }

    /// Return the location path as raw node indices.
    #[must_use]
    pub fn as_usize_vec(&self) -> Vec<usize> {
        self.0
            .iter()
            .map(|component| match component {
                LocationComponent::Node(index) => index.index(),
            })
            .collect()
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let serialized =
            serde_json::to_string(&self.as_usize_vec()).map_err(|_| std::fmt::Error)?;
        write!(f, "{serialized}")
    }
}

impl FromStr for Location {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let nodes = serde_json::from_str::<Vec<usize>>(s)?;
        Ok(Self::from_usize_iter(nodes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_location_serialization() {
        let location = Location::from_usize_iter([2, 4, 8]);
        let serialized = location.to_string();
        let parsed = serialized.parse::<Location>().unwrap();
        assert_eq!(location, parsed);
    }

    #[test]
    fn root_serializes_as_empty_path() {
        let root = Location::root();
        assert_eq!(root.to_string(), "[]");
        assert_eq!("[]".parse::<Location>().unwrap(), root);
    }
}
