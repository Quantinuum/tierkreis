/*!
This module defines the [`FileAssetStorage`] struct which implements [`AssetStorage`]
by storing files in a single directory.
*/

use std::path::{Path, PathBuf};

use futures::future::{BoxFuture, FutureExt};
use miette::{Context, IntoDiagnostic, miette};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::asset_storage::interface::{AssetKey, AssetKind, AssetStorage};

/// [`FileAssetStorage`] is an implementation of [`AssetStorage`] that stores
/// Assets in a single directory using file names derived from [`AssetKey`]s.
#[derive(Clone, Debug)]
pub struct FileAssetStorage {
    base_dir: PathBuf,
}

impl FileAssetStorage {
    /// Create a new [`FileAssetStorage`] backed by a folder define by `path`.
    #[must_use]
    pub fn new(path: &Path) -> Self {
        Self {
            base_dir: path.to_path_buf(),
        }
    }

    fn location(&self, key: &AssetKey) -> PathBuf {
        self.base_dir.join(key.to_string())
    }
}

impl AssetStorage for FileAssetStorage {
    fn kind(&self) -> AssetKind {
        AssetKind::File {
            root: self.base_dir.clone(),
        }
    }

    fn exists(&self, key: &AssetKey) -> BoxFuture<'_, miette::Result<bool>> {
        let location = self.location(key);
        async move {
            tokio::fs::try_exists(&location)
                .await
                .into_diagnostic()
                .wrap_err_with(|| {
                    miette!("Could not determine whether file exists at location: {location:?}")
                })
        }
        .boxed()
    }

    fn save(&self, key: &AssetKey, value: Vec<u8>) -> BoxFuture<'_, miette::Result<()>> {
        let location = self.location(key);
        async move {
            let mut file = File::create(&location)
                .await
                .into_diagnostic()
                .wrap_err_with(|| miette!("Cannot find file at location: {location:?}"))?;

            file.write_all(&value).await.into_diagnostic()?;

            Ok(())
        }
        .boxed()
    }

    fn load(&self, key: &AssetKey) -> BoxFuture<'_, miette::Result<Vec<u8>>> {
        let location = self.location(key);
        async move {
            let mut file = File::open(&location)
                .await
                .into_diagnostic()
                .wrap_err_with(|| miette!("Cannot find file at location: {location:?}"))?;

            let mut value = Vec::new();
            file.read_to_end(&mut value).await.into_diagnostic()?;

            Ok(value)
        }
        .boxed()
    }
}
