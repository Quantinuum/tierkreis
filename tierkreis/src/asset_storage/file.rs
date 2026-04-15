use std::{
    fs::File,
    path::{Path, PathBuf},
};

use miette::{Context, IntoDiagnostic, miette};
use serde_json::Value;

use crate::asset_storage::interface::{AssetKey, AssetKind, AssetStorage};

#[derive(Clone, Debug)]
pub struct FileAssetStorage {
    base_dir: PathBuf,
}

impl FileAssetStorage {
    pub fn new(path: &Path) -> Self {
        Self {
            base_dir: path.to_path_buf(),
        }
    }

    fn location(&self, key: &AssetKey) -> PathBuf {
        let mut path = self.base_dir.clone();
        path.push(key.0.to_string());
        path
    }
}

impl AssetStorage for FileAssetStorage {
    fn kind(&self) -> AssetKind {
        AssetKind::File {
            parent: self.base_dir.clone(),
        }
    }

    fn exists(&self, key: &AssetKey) -> miette::Result<bool> {
        let location = self.location(key);
        location.try_exists().into_diagnostic().wrap_err(miette!(
            "Could not determine whether file exists at location: {location:?}"
        ))
    }

    fn save(&self, key: &AssetKey, value: Value) -> miette::Result<()> {
        let location = self.location(key);
        let mut file = File::create(self.location(key))
            .into_diagnostic()
            .wrap_err(miette!("Cannot find file at location: {location:?}"))?;
        serde_json::to_writer(&mut file, &value).into_diagnostic()?;
        Ok(())
    }

    fn load(&self, key: &AssetKey) -> miette::Result<Value> {
        let location = self.location(key);
        let file = File::open(self.location(key))
            .into_diagnostic()
            .wrap_err(miette!("Cannot find file at location: {location:?}"))?;
        let value: Value = serde_json::from_reader(&file).into_diagnostic()?;
        Ok(value)
    }
}
