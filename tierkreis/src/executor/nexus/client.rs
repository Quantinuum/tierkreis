pub(crate) mod models;

use std::{
    env::home_dir,
    fmt::Display,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use futures::{
    SinkExt, Stream, StreamExt,
    channel::mpsc,
    stream::{SplitSink, SplitStream},
};
use hugr::package::Package;
use miette::{IntoDiagnostic, miette};
use reqwest::{Client, ClientBuilder, cookie::Jar};
use reqwest_websocket::{Bytes, Message, Upgrade};
use serde::Deserialize;
use tokio::{fs::File, io::AsyncReadExt, task::JoinHandle};
use tracing::warn;
use url::{Host, Url};
use uuid::Uuid;

use crate::executor::nexus::client::models::{
    CollectionDocument, Data, Document, NewExecuteJobItem, NewHugr, NewJob, NewJobDefinition,
    NewProject,
    jobs::{Job, JobData, Status},
    results::{QSysResult, QSysResultData},
};

#[derive(Deserialize)]
struct AccessToken {
    data: AccessTokenData,
}

#[derive(Deserialize)]
struct AccessTokenData {
    access_token: String,
}

#[derive(Deserialize)]
struct RefreshToken {
    data: RefreshTokenData,
}

#[derive(Deserialize)]
struct RefreshTokenData {
    #[allow(unused)]
    delete_version_after: Option<String>,
    refresh_token: String,
}

pub struct JobStatusStream {
    stream: SplitStream<reqwest_websocket::WebSocket>,
    join_handle: JoinHandle<miette::Result<SplitSink<reqwest_websocket::WebSocket, Message>>>,
    close_sender: mpsc::Sender<()>,
}

impl JobStatusStream {
    fn new(websocket: reqwest_websocket::WebSocket) -> Self {
        let (mut sink, stream) = websocket.split();
        let (close_sender, mut close_receiver) = mpsc::channel(1);
        let join_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(55));
            loop {
                tokio::select! {
                    _ = close_receiver.recv() => {
                        return Ok(sink)
                    }
                    _ = interval.tick() => {
                        sink.send(reqwest_websocket::Message::Ping(Bytes::new()))
                            .await.into_diagnostic()?;
                    }
                }
            }
        });
        Self {
            stream,
            join_handle,
            close_sender,
        }
    }
}

impl Stream for JobStatusStream {
    type Item = miette::Result<Status>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let next = self.get_mut().stream.poll_next_unpin(cx);
        match next {
            Poll::Ready(Some(Ok(Message::Text(text)))) => {
                Poll::Ready(Some(serde_json::from_str(&text).into_diagnostic()))
            }
            Poll::Ready(Some(Ok(Message::Binary(bin)))) => {
                Poll::Ready(Some(serde_json::from_slice(&bin).into_diagnostic()))
            }
            // Ignoring ping messages may be sub-optimal, we may want to have a way to send
            // pong responses but this would require another channel and a bit more
            // engineering effort.
            Poll::Ready(Some(Ok(Message::Ping(_)))) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            // We are ignoring pong messages we receive, which seems reasonable to
            // abstract over as there are no further actions to take.
            Poll::Ready(Some(Ok(Message::Pong(_)))) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            // We may want to reconnect in this case, but that is a decision for the consumer
            // of this stream to make.
            Poll::Ready(Some(Ok(Message::Close { code, reason }))) => {
                warn!("Websocket closed by peer with code: {code}, reason: {reason}");
                Poll::Ready(None)
            }
            Poll::Ready(Some(Err(err))) => {
                Poll::Ready(Some(Err(miette!("Websocket error: {err}"))))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

impl JobStatusStream {
    pub async fn close(mut self) -> miette::Result<()> {
        self.close_sender.send(()).await.into_diagnostic()?;
        let sink = self.join_handle.await.into_diagnostic()??;
        let websocket = self.stream.reunite(sink).into_diagnostic()?;
        websocket
            .close(reqwest_websocket::CloseCode::Normal, None)
            .await
            .into_diagnostic()?;
        Ok(())
    }
}

pub enum Scheme {
    Http,
    Https,
}

impl Display for Scheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http => write!(f, "http"),
            Self::Https => write!(f, "https"),
        }
    }
}

#[derive(Clone)]
pub struct NexusClient {
    // See https://github.com/jgraef/reqwest-websocket/issues/2
    // for why we need a separate Client instance for websockets.
    http1_client: Client,
    client: Client,
    base_url: Url,
}

impl NexusClient {
    pub async fn try_new(
        scheme: Scheme,
        host: Host,
        token_dir: Option<&Path>,
    ) -> miette::Result<Self> {
        // Assumes that a user has credentials saved from qnexus in their home directory.
        //
        // While we could replicate the device code login flow here, this is much easier
        // for getting something running in the short term.
        let token_dir_path = token_dir.map_or_else(
            || -> miette::Result<PathBuf> {
                let home_dir_path = home_dir().ok_or_else(|| miette!("no home directory"))?;
                let token_dir_path = home_dir_path.join(".qnx/auth");
                Ok(token_dir_path)
            },
            |path| Ok(path.to_path_buf()),
        )?;
        let access_token_path = token_dir_path.join("id.json");
        let refresh_token_path = token_dir_path.join("token.json");

        let mut access_token_file = File::open(access_token_path).await.into_diagnostic()?;
        let mut refresh_token_file = File::open(refresh_token_path).await.into_diagnostic()?;

        let mut access_token_contents = String::new();
        access_token_file
            .read_to_string(&mut access_token_contents)
            .await
            .into_diagnostic()?;
        let access_token: AccessToken =
            serde_json::from_str(&access_token_contents).into_diagnostic()?;

        let mut refresh_token_contents = String::new();
        refresh_token_file
            .read_to_string(&mut refresh_token_contents)
            .await
            .into_diagnostic()?;
        let refresh_token: RefreshToken =
            serde_json::from_str(&refresh_token_contents).into_diagnostic()?;

        let base_url: Url = format!("{scheme}://{host}").parse().into_diagnostic()?;
        let jar = Jar::default();
        jar.add_cookie_str(
            &format!("myqos_oat={}", refresh_token.data.refresh_token),
            &base_url,
        );
        jar.add_cookie_str(
            &format!("myqos_id={}", access_token.data.access_token),
            &base_url,
        );

        let jar = Arc::new(jar);

        let http1_client = ClientBuilder::new()
            .cookie_provider(Arc::clone(&jar))
            .http1_only() // See https://github.com/jgraef/reqwest-websocket/issues/2
            .build()
            .into_diagnostic()?;

        let client = ClientBuilder::new()
            .cookie_provider(jar)
            .build()
            .into_diagnostic()?;

        Ok(Self {
            http1_client,
            client,
            base_url,
        })
    }

    pub async fn refresh_tokens(&self) -> miette::Result<()> {
        let url = self
            .base_url
            .join("/auth/tokens/refresh")
            .into_diagnostic()?;
        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .send()
            .await
            .into_diagnostic()?;

        response.error_for_status_ref().into_diagnostic()?;

        Ok(())
    }

    pub async fn find_project_data(&self, name: &str) -> miette::Result<Option<Data>> {
        let url = self
            .base_url
            .join("/api/projects/v1beta2")
            .into_diagnostic()?;
        let response = self
            .client
            .get(url)
            .query(&[("filter[name_exact]", name)])
            .send()
            .await
            .into_diagnostic()?;

        response.error_for_status_ref().into_diagnostic()?;
        let projects: CollectionDocument = response.json().await.into_diagnostic()?;

        // We expect exactly one element
        Ok(projects.last_data())
    }

    pub async fn new_project_data(
        &self,
        name: &str,
        description: Option<&str>,
    ) -> miette::Result<Data> {
        let url = self
            .base_url
            .join("/api/projects/v1beta2")
            .into_diagnostic()?;
        let response = self
            .client
            .post(url)
            .json(&NewProject::new(name, description))
            .send()
            .await
            .into_diagnostic()?;

        response.error_for_status_ref().into_diagnostic()?;
        let project: Document = response.json().await.into_diagnostic()?;
        Ok(project.data())
    }

    pub async fn find_or_create_project_data(
        &self,
        name: &str,
        description: Option<&str>,
    ) -> miette::Result<Data> {
        let project_data = self.find_project_data(name).await?;
        if let Some(project_data) = project_data {
            Ok(project_data)
        } else {
            self.new_project_data(name, description).await
        }
    }

    pub async fn new_hugr_data(
        &self,
        name: &str,
        description: Option<&str>,
        project_id: Uuid,
        package: Package,
    ) -> miette::Result<Data> {
        let url = self.base_url.join("/api/hugr/v1beta").into_diagnostic()?;
        let response = self
            .client
            .post(url)
            .json(&NewHugr::new(name, description, project_id, package))
            .send()
            .await
            .into_diagnostic()?;

        response.error_for_status_ref().into_diagnostic()?;
        let hugr: Document = response.json().await.into_diagnostic()?;
        Ok(hugr.data())
    }

    pub async fn new_job_data(
        &self,
        name: &str,
        description: Option<&str>,
        project_id: Uuid,
        hugr_ids: impl IntoIterator<Item = (Uuid, u64)>,
    ) -> miette::Result<Data> {
        let url = self.base_url.join("/api/jobs/v1beta3").into_diagnostic()?;
        let items: Vec<_> = hugr_ids
            .into_iter()
            .map(|(hugr_id, n_shots)| NewExecuteJobItem::new(hugr_id, n_shots))
            .collect();

        let response = self
            .client
            .post(url)
            .json(&NewJob::new(
                name,
                description,
                project_id,
                NewJobDefinition::new_execute(&items),
            ))
            .send()
            .await
            .into_diagnostic()?;

        response.error_for_status_ref().into_diagnostic()?;
        let job: Document = response.json().await.into_diagnostic()?;
        Ok(job.data())
    }

    pub async fn get_job(&self, job_id: Uuid) -> miette::Result<JobData> {
        let url = self
            .base_url
            .join("/api/jobs/v1beta3/")
            .and_then(|url| url.join(&job_id.to_string()))
            .into_diagnostic()?;
        let response = self.client.get(url).send().await.into_diagnostic()?;

        response.error_for_status_ref().into_diagnostic()?;
        let job: Job = response.json().await.into_diagnostic()?;
        Ok(job.data())
    }

    pub async fn listen_for_job_status(&self, job_id: Uuid) -> miette::Result<JobStatusStream> {
        let response = self
            .http1_client
            .get(format!(
                "wss://nexus.quantinuum.com/api/jobs/v1beta3/{job_id}/attributes/status/ws"
            ))
            .upgrade()
            .send()
            .await
            .into_diagnostic()?;

        let websocket = response.into_websocket().await.into_diagnostic()?;
        Ok(JobStatusStream::new(websocket))
    }

    pub async fn get_qsys_result_chunk(
        &self,
        result_id: Uuid,
        chunk_number: u32,
    ) -> miette::Result<QSysResultData> {
        let url = self
            .base_url
            .join("/api/qsys_results/v1beta2/partial/")
            .and_then(|url| url.join(&result_id.to_string()))
            .into_diagnostic()?;
        let response = self
            .client
            .get(url)
            .query(&[("chunk_number", chunk_number)])
            .send()
            .await
            .into_diagnostic()?;

        response.error_for_status_ref().into_diagnostic()?;
        let result: QSysResult = response.json().await.into_diagnostic()?;
        Ok(result.data())
    }
}

#[cfg(test)]
mod tests {
    use mockito::Matcher::{AnyOf, PartialJsonString};
    use tempfile::{TempDir, tempdir};
    use tokio::io::AsyncWriteExt;

    use super::*;

    async fn setup_temp_tokens() -> miette::Result<TempDir> {
        let token_dir = tempdir().into_diagnostic()?;
        let token_dir_path = token_dir.path();

        let access_token_path = token_dir_path.join("id.json");
        let refresh_token_path = token_dir_path.join("token.json");

        let mut access_token_file = File::create(access_token_path).await.into_diagnostic()?;
        access_token_file
            .write_all(b"{\"data\": {\"access_token\": \"YWJj\"}}")
            .await
            .into_diagnostic()?;

        let mut refresh_token_file = File::create(refresh_token_path).await.into_diagnostic()?;
        refresh_token_file
            .write_all(b"{\"data\": {\"refresh_token\": \"Y2Jh\"}}")
            .await
            .into_diagnostic()?;

        Ok(token_dir)
    }

    #[tokio::test]
    async fn refresh_tokens() -> miette::Result<()> {
        let token_dir = setup_temp_tokens().await?;
        let token_path = token_dir.path();

        let mut server = mockito::Server::new_async().await;
        let host = Host::Domain(server.host_with_port());

        let mock = server
            .mock("POST", "/auth/tokens/refresh")
            .with_status(201)
            // Ordering is semi-random from the jar storage.
            .match_header(
                "Cookie",
                AnyOf(vec![
                    "myqos_oat=Y2Jh; myqos_id=YWJj".into(),
                    "myqos_id=YWJj; myqos_oat=Y2Jh".into(),
                ]),
            )
            .create();

        let client = NexusClient::try_new(Scheme::Http, host, Some(token_path)).await?;
        client.refresh_tokens().await?;

        mock.assert_async().await;

        Ok(())
    }

    #[tokio::test]
    async fn refresh_tokens_error() -> miette::Result<()> {
        let token_dir = setup_temp_tokens().await?;
        let token_path = token_dir.path();

        let mut server = mockito::Server::new_async().await;
        let host = Host::Domain(server.host_with_port());

        let mock = server
            .mock("POST", "/auth/tokens/refresh")
            .with_status(401)
            .create();

        let client = NexusClient::try_new(Scheme::Http, host, Some(token_path)).await?;
        let err = client.refresh_tokens().await.unwrap_err();
        assert!(
            err.to_string()
                .contains("HTTP status client error (401 Unauthorized)")
        );

        mock.assert_async().await;

        Ok(())
    }

    #[tokio::test]
    async fn find_or_create_project_find() -> miette::Result<()> {
        let token_dir = setup_temp_tokens().await?;
        let token_path = token_dir.path();

        let mut server = mockito::Server::new_async().await;
        let host = Host::Domain(server.host_with_port());

        let mock = server
            .mock("GET", "/api/projects/v1beta2?filter%5Bname_exact%5D=foo")
            .with_status(200)
            .with_body("{\"data\": [{\"id\": \"ebdc7a71-45d7-4a8f-b175-1361903a760b\"}]}")
            .create();

        let client = NexusClient::try_new(Scheme::Http, host, Some(token_path)).await?;
        let project_data = client
            .find_or_create_project_data("foo", Some("description"))
            .await?;

        assert_eq!(
            project_data.id().to_string(),
            "ebdc7a71-45d7-4a8f-b175-1361903a760b"
        );

        mock.assert_async().await;

        Ok(())
    }

    #[tokio::test]
    async fn find_or_create_project_create() -> miette::Result<()> {
        let token_dir = setup_temp_tokens().await?;
        let token_path = token_dir.path();

        let mut server = mockito::Server::new_async().await;
        let host = Host::Domain(server.host_with_port());

        let mock1 = server
            .mock("GET", "/api/projects/v1beta2?filter%5Bname_exact%5D=foo")
            .with_status(200)
            .with_body("{\"data\": []}")
            .create();

        let mock2 = server
            .mock("POST", "/api/projects/v1beta2")
            .with_status(201)
            .with_body("{\"data\": {\"id\": \"ebdc7a71-45d7-4a8f-b175-1361903a760b\"}}")
            .match_body(PartialJsonString(
                "{
                    \"data\": {
                        \"attributes\": {
                            \"name\": \"foo\",
                            \"description\": \"description\",
                            \"properties\": {}
                        },
                        \"relationships\": {},
                        \"type\": \"project\"
                    }
                }"
                .to_string(),
            ))
            .create();

        let client = NexusClient::try_new(Scheme::Http, host, Some(token_path)).await?;
        let project_data = client
            .find_or_create_project_data("foo", Some("description"))
            .await?;

        assert_eq!(
            project_data.id().to_string(),
            "ebdc7a71-45d7-4a8f-b175-1361903a760b"
        );

        mock1.assert_async().await;
        mock2.assert_async().await;

        Ok(())
    }
}
