use std::{path::PathBuf, time::SystemTime};

use miette::miette;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub enum AssetKind {
    Memory,
    File { parent: PathBuf },
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssetSpec {
    pub kind: AssetKind,
    pub storage_name: String,
    pub asset_key: AssetKey,
}

impl AssetSpec {
    // Return a filesystem path if the Asset is of Kind File.
    pub fn path(&self) -> miette::Result<PathBuf> {
        match &self.kind {
            AssetKind::File { parent } => Ok(parent.join(self.asset_key.0.to_string())),
            _ => Err(miette!("Cannot build a path from a non-File asset!")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetKey(pub Uuid);

impl AssetKey {
    pub fn new() -> Self {
        let timestamp = SystemTime::now().try_into().unwrap();
        Self(uuid::Uuid::new_v7(timestamp))
    }
}

impl Default for AssetKey {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for AssetKey {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl TryFrom<&str> for AssetKey {
    type Error = uuid::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(Self(value.parse()?))
    }
}

pub trait AssetStorage: Send + Sync {
    fn kind(&self) -> AssetKind;
    fn exists(&self, key: &AssetKey) -> miette::Result<bool>;
    fn save(&self, key: &AssetKey, value: Value) -> miette::Result<()>;
    fn load(&self, key: &AssetKey) -> miette::Result<Value>;
}
