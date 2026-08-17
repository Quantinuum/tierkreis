/*!
This module defines the central logging and tracing capabilities of the runtime.
*/
use opentelemetry::propagation::TextMapCompositePropagator;
use opentelemetry::{KeyValue, global, trace::TracerProvider};
use opentelemetry_otlp::{ExporterBuildError, WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::propagation::{BaggagePropagator, TraceContextPropagator};
use opentelemetry_sdk::trace::{SdkTracerProvider, Tracer};
use serde::Deserialize;
use std::{
    env::home_dir,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;

/// The log format to use for the runtime.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum LogFormat {
    Json,
    Pretty,
    Compact,
}

/// The logging configuration for the runtime.
#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    log_file: Option<PathBuf>,
    log_format: LogFormat,
    log_level: Option<String>,

    otel_endpoint: Option<String>,
    service_name: Option<String>,
    service_namespace: Option<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        let tierkreis_log = home_dir()
            .unwrap_or_else(|| "/tmp".into())
            .join(".tierkreis/tierkreis.log");

        let otel_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();

        LoggingConfig {
            log_file: Some(tierkreis_log),
            log_format: LogFormat::Compact,
            log_level: Some("info".to_string()),
            otel_endpoint,
            service_name: Some("tierkreis".to_string()),
            service_namespace: None,
        }
    }
}

static LOG_GUARD: Mutex<Option<WorkerGuard>> = Mutex::new(None);
static LOGGING_INITIALIZED: OnceLock<()> = OnceLock::new();
static TRACER_PROVIDER: Mutex<Option<SdkTracerProvider>> = Mutex::new(None);

/// Flush buffered log lines to the log file and shut down OpenTelemetry.
///
/// The non-blocking appender only drains when its guard is dropped, so this
/// consumes the guard. Also shuts down the tracer provider to flush any
/// pending spans.
pub fn flush_logs() {
    if let Ok(mut guard) = LOG_GUARD.lock() {
        drop(guard.take());
    }
    // Shutdown the tracer provider to ensure all spans are flushed
    if let Ok(mut provider) = TRACER_PROVIDER.lock() {
        if let Some(p) = provider.take() {
            let _ = p.shutdown();
        }
    }
}

macro_rules! with_format_layer {
    ($log_format:expr, $writer:expr, |$fmt_layer:ident| $body:block) => {
        match $log_format {
            LogFormat::Json => {
                let $fmt_layer = fmt::layer().json().with_writer($writer);
                $body
            }
            LogFormat::Pretty => {
                let $fmt_layer = fmt::layer().pretty().with_writer($writer);
                $body
            }
            LogFormat::Compact => {
                let $fmt_layer = fmt::layer().compact().with_writer($writer);
                $body
            }
        }
    };
}

fn init_tracing_layer<S>(
    otlp: &str,
    resource: &Resource,
) -> Result<OpenTelemetryLayer<S, Tracer>, ExporterBuildError>
where
    for<'s> S: LookupSpan<'s> + tracing::Subscriber,
{
    global::set_text_map_propagator(TextMapCompositePropagator::new(vec![
        Box::new(TraceContextPropagator::new()),
        Box::new(BaggagePropagator::new()),
    ]));
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_compression(opentelemetry_otlp::Compression::Gzip)
        .with_endpoint(otlp)
        .build()?;

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource.clone())
        .build();

    global::set_tracer_provider(provider.clone());

    // Store the provider so we can shutdown later
    if let Ok(mut stored_provider) = TRACER_PROVIDER.lock() {
        *stored_provider = Some(provider.clone());
    }

    Ok(tracing_opentelemetry::layer().with_tracer(provider.tracer("tierkreis")))
}

fn init_metrics_layer<S>(
    otlp: &str,
    resource: &Resource,
) -> Result<MetricsLayer<S, SdkMeterProvider>, ExporterBuildError>
where
    for<'s> S: LookupSpan<'s> + tracing::Subscriber,
{
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_compression(opentelemetry_otlp::Compression::Gzip)
        .with_endpoint(otlp)
        .build()?;

    let provider = SdkMeterProvider::builder()
        .with_resource(resource.clone())
        .with_periodic_exporter(exporter)
        .build();

    global::set_meter_provider(provider.clone());

    Ok(MetricsLayer::new(provider))
}

fn log_filter(log_level: Option<&str>) -> EnvFilter {
    // EnvFilter::try_from_default_env()
    EnvFilter::new(format!("tierkreis={}", log_level.unwrap_or("info")))
}

fn service_resource(config: &LoggingConfig) -> Resource {
    Resource::builder()
        .with_attributes(vec![
            KeyValue::new(
                "service.name",
                config
                    .service_name
                    .clone()
                    .unwrap_or_else(|| "tierkreis".to_string()),
            ),
            KeyValue::new(
                "service.namespace",
                config
                    .service_namespace
                    .clone()
                    .unwrap_or_else(|| "tierkreis".to_string()),
            ),
        ])
        .build()
}

fn make_writer(path: Option<&Path>) -> BoxMakeWriter {
    if let Some(path) = path {
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        let file = path.file_name().unwrap_or_else(|| "tierkreis.log".as_ref());
        let appender = tracing_appender::rolling::never(dir, file);
        let (non_blocking, guard): (NonBlocking, WorkerGuard) =
            tracing_appender::non_blocking(appender);
        LOG_GUARD.lock().expect("log guard poisoned").replace(guard);
        BoxMakeWriter::new(non_blocking)
    } else {
        BoxMakeWriter::new(std::io::stderr)
    }
}

fn init(config: &LoggingConfig, with_telemetry: bool) {
    LOGGING_INITIALIZED.get_or_init(|| {
        let filter = log_filter(config.log_level.as_deref());
        let writer = make_writer(config.log_file.as_deref());

        if with_telemetry && let Some(otlp) = config.otel_endpoint.as_deref() {
            let resource = service_resource(config);
            with_format_layer!(&config.log_format, writer, |fmt_layer| {
                let tracing = init_tracing_layer(otlp, &resource).expect("Failed to init tracer.");
                let metrics = init_metrics_layer(otlp, &resource).expect("Failed to init metrics.");
                let subscriber = tracing_subscriber::registry()
                    .with(filter)
                    .with(fmt_layer)
                    .with(tracing)
                    .with(metrics);
                tracing::subscriber::set_global_default(subscriber)
                    .expect("Setting default subscriber failed");
            });
            return;
        }

        with_format_layer!(&config.log_format, writer, |fmt_layer| {
            let subscriber = tracing_subscriber::registry().with(filter).with(fmt_layer);
            tracing::subscriber::set_global_default(subscriber)
                .expect("Setting default subscriber failed");
        });
    });
}

/// Initialize logging without OpenTelemetry.
pub fn init_logging(logging_config: Option<LoggingConfig>) {
    init(&logging_config.unwrap_or_default(), false);
}

/// Initialize the runtime subscriber with logging and OpenTelemetry.
pub fn init_logging_and_tracing(logging_config: Option<LoggingConfig>) {
    init(&logging_config.unwrap_or_default(), true);
}
