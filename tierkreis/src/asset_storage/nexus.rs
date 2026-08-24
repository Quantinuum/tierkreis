//! Nexus project-backed Asset storage.

use std::{io::Cursor, sync::Arc};

use dashmap::DashMap;
use futures::{FutureExt, future::BoxFuture};
use hugr::{envelope::read_envelope, extension::ExtensionRegistry};
use miette::{IntoDiagnostic, WrapErr, miette};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    asset_storage::{AssetDataType, AssetKey, AssetKind, AssetStorage},
    executor::nexus::client::{NexusClient, NexusClientConfig},
    state::{AssetLocation, RuntimeState},
};

const LOCATION_TYPE: &str = "nexus-resource";
const LOCATION_SCHEMA_VERSION: u32 = 1;

/// Resource types supported by [`NexusProjectAssetStorage`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NexusResourceKind {
    /// A HUGR program.
    Hugr,
    /// A pytket circuit.
    Circuit,
    /// A quantum execution result.
    ExecutionResult,
}

impl NexusResourceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Hugr => "hugr",
            Self::Circuit => "circuit",
            Self::ExecutionResult => "execution-result",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct NexusAssetLocator {
    project_id: Uuid,
    resource_kind: NexusResourceKind,
    resource_id: Uuid,
}

/// An Asset storage backed by resources in one Nexus project.
#[derive(Clone)]
pub struct NexusProjectAssetStorage {
    client: NexusClient,
    runtime_state: Arc<dyn RuntimeState>,
    storage_name: String,
    project_name: String,
    project_id: Uuid,
    locks: Arc<DashMap<AssetKey, Arc<Mutex<()>>>>,
}

impl std::fmt::Debug for NexusProjectAssetStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NexusProjectAssetStorage")
            .field("storage_name", &self.storage_name)
            .field("project_name", &self.project_name)
            .field("project_id", &self.project_id)
            .finish_non_exhaustive()
    }
}

impl NexusProjectAssetStorage {
    /// Create storage for a named Nexus project, creating the project if necessary.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication, project lookup, or project creation fails.
    pub async fn try_new(
        client_config: &NexusClientConfig,
        runtime_state: Arc<dyn RuntimeState>,
        storage_name: impl Into<String>,
        project_name: impl Into<String>,
    ) -> miette::Result<Self> {
        let storage_name = storage_name.into();
        let project_name = project_name.into();
        let client = NexusClient::try_new(client_config).await?;
        client.refresh_tokens().await?;
        let project = client
            .find_or_create_project_data(&project_name, Some("Tierkreis runtime assets"))
            .await?;
        Ok(Self {
            client,
            runtime_state,
            storage_name,
            project_name,
            project_id: project.id(),
            locks: Arc::new(DashMap::new()),
        })
    }

    pub(crate) fn client(&self) -> NexusClient {
        self.client.clone()
    }

    pub(crate) fn project_id(&self) -> Uuid {
        self.project_id
    }

    pub(crate) fn project_name(&self) -> &str {
        &self.project_name
    }

    pub(crate) fn storage_name(&self) -> &str {
        &self.storage_name
    }

    async fn locator(&self, key: AssetKey) -> miette::Result<Option<NexusAssetLocator>> {
        let Some(location) = self
            .runtime_state
            .load_asset_location(key, &self.storage_name)
            .await?
        else {
            return Ok(None);
        };
        if location.location_type != LOCATION_TYPE
            || location.schema_version != LOCATION_SCHEMA_VERSION
        {
            return Err(miette!(
                "Unsupported Nexus Asset location type/version: {}/{}",
                location.location_type,
                location.schema_version
            ));
        }
        let locator: NexusAssetLocator = serde_json::from_value(location.data).into_diagnostic()?;
        if locator.project_id != self.project_id {
            return Err(miette!("Nexus Asset belongs to a different project"));
        }
        Ok(Some(locator))
    }

    async fn bind(&self, key: AssetKey, locator: NexusAssetLocator) -> miette::Result<()> {
        self.runtime_state
            .put_asset_location(
                key,
                AssetLocation {
                    storage_name: self.storage_name.clone(),
                    location_type: LOCATION_TYPE.to_string(),
                    schema_version: LOCATION_SCHEMA_VERSION,
                    data: serde_json::to_value(locator).into_diagnostic()?,
                },
            )
            .await
    }

    pub(crate) async fn bind_execution_result(
        &self,
        key: AssetKey,
        result_id: Uuid,
    ) -> miette::Result<()> {
        self.bind(
            key,
            NexusAssetLocator {
                project_id: self.project_id,
                resource_kind: NexusResourceKind::ExecutionResult,
                resource_id: result_id,
            },
        )
        .await
    }

    pub(crate) async fn resource_id(
        &self,
        key: AssetKey,
        expected_kind: NexusResourceKind,
    ) -> miette::Result<Uuid> {
        let locator = self
            .locator(key)
            .await?
            .ok_or_else(|| miette!("No Nexus location registered for Asset {key}"))?;
        if locator.resource_kind != expected_kind {
            return Err(miette!("Unexpected Nexus resource kind"));
        }
        Ok(locator.resource_id)
    }

    async fn save_resource(
        &self,
        key: AssetKey,
        value: Vec<u8>,
        resource_kind: NexusResourceKind,
    ) -> miette::Result<()> {
        let lock = self
            .locks
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        let _guard = lock.lock().await;
        if let Some(locator) = self.locator(key).await? {
            if locator.resource_kind == resource_kind {
                return Ok(());
            }
            return Err(miette!(
                "Asset is already registered with another Nexus resource kind"
            ));
        }

        self.client.refresh_tokens().await?;
        let name = format!("tierkreis-{key}");
        let resource_id = match resource_kind {
            NexusResourceKind::Hugr => {
                let (_, package) = read_envelope(Cursor::new(value), &ExtensionRegistry::new([]))
                    .into_diagnostic()
                    .wrap_err("Failed to decode HUGR Asset")?;
                self.client
                    .new_hugr_data(&name, None, self.project_id, package)
                    .await?
                    .id()
            }
            NexusResourceKind::Circuit => {
                let circuit = serde_json::from_slice(&value).into_diagnostic()?;
                self.client
                    .new_circuit_data(&name, self.project_id, circuit)
                    .await?
                    .id()
            }
            NexusResourceKind::ExecutionResult => {
                return Err(miette!(
                    "Execution results must be bound from a completed job"
                ));
            }
        };
        self.bind(
            key,
            NexusAssetLocator {
                project_id: self.project_id,
                resource_kind,
                resource_id,
            },
        )
        .await
    }
}

impl AssetStorage for NexusProjectAssetStorage {
    fn kind(&self) -> AssetKind {
        AssetKind::NexusProject {
            project_id: self.project_id,
        }
    }

    fn exists(&self, key: &AssetKey) -> BoxFuture<'_, miette::Result<bool>> {
        let key = *key;
        async move {
            let Some(locator) = self.locator(key).await? else {
                return Ok(false);
            };
            self.client.refresh_tokens().await?;
            self.client
                .resource_exists(locator.resource_kind.as_str(), locator.resource_id)
                .await
        }
        .boxed()
    }

    fn save(&self, _key: &AssetKey, _value: Vec<u8>) -> BoxFuture<'_, miette::Result<()>> {
        async {
            Err(miette!(
                "Nexus Assets must be saved with a HUGR or Circuit data type"
            ))
        }
        .boxed()
    }

    fn save_typed<'a>(
        &'a self,
        key: &'a AssetKey,
        value: Vec<u8>,
        data_type: AssetDataType,
    ) -> BoxFuture<'a, miette::Result<()>> {
        let key = *key;
        async move {
            let kind = match data_type {
                AssetDataType::Hugr => NexusResourceKind::Hugr,
                AssetDataType::Circuit => NexusResourceKind::Circuit,
                AssetDataType::Opaque => {
                    return Err(miette!("Nexus Assets require a semantic data type"));
                }
            };
            self.save_resource(key, value, kind).await
        }
        .boxed()
    }

    fn load(&self, key: &AssetKey) -> BoxFuture<'_, miette::Result<Vec<u8>>> {
        let key = *key;
        async move {
            let locator = self
                .locator(key)
                .await?
                .ok_or_else(|| miette!("No Nexus location registered for Asset {key}"))?;
            self.client.refresh_tokens().await?;
            match locator.resource_kind {
                NexusResourceKind::Hugr => self.client.get_hugr_bytes(locator.resource_id).await,
                NexusResourceKind::Circuit => {
                    self.client.get_circuit_bytes(locator.resource_id).await
                }
                NexusResourceKind::ExecutionResult => {
                    let mut full_result = Vec::new();
                    let mut chunk_number = 0;
                    while let Some(result) = self
                        .client
                        .get_qsys_result_chunk(locator.resource_id, chunk_number)
                        .await?
                    {
                        full_result.extend(result.results());
                        chunk_number += 1;
                    }
                    serde_json::to_vec(&full_result).into_diagnostic()
                }
            }
        }
        .boxed()
    }
}
