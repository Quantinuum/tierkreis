/*!
This module defines the central logging capabilities of the runtime.
*/

use std::{path::Path, sync::OnceLock};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::Resource;
use opentelemetry::propagation::TextMapCompositePropagator;
use opentelemetry::{KeyValue, global, trace::TracerProvider};
use opentelemetry_otlp::{ExporterBuildError, WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::propagation::{BaggagePropagator, TraceContextPropagator};
use opentelemetry_sdk::trace::{SdkTracerProvider, Tracer};
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt as _;
use tracing_subscriber::fmt;

use crate::runtime::{LogFormat, LoggingConfig};

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();
static TRACER_PROVIDER: OnceLock<SdkTracerProvider> = OnceLock::new();

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
    let _ = TRACER_PROVIDER.set(provider.clone());

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
    EnvFilter::new(&format!("tierkreis={}", log_level.unwrap_or("info")))
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
        let (non_blocking, guard): (NonBlocking, WorkerGuard) = tracing_appender::non_blocking(appender);
        LOG_GUARD.set(guard).expect("log guard already set");
        BoxMakeWriter::new(non_blocking)
    } else {
        BoxMakeWriter::new(std::io::stderr)
    }
}

fn init(config: LoggingConfig, with_telemetry: bool) {
    let filter = log_filter(config.log_level.as_deref());
    let writer = make_writer(config.log_file.as_deref());

    if with_telemetry {
        if let Some(otlp) = config.otel_endpoint.as_deref() {
            let resource = service_resource(&config);
            with_format_layer!(&config.log_format, writer, |fmt_layer| {
                let tracing = init_tracing_layer(otlp, &resource).expect("Failed to init tracer.");
                let metrics = init_metrics_layer(otlp, &resource).expect("Failed to init metrics.");
                tracing_subscriber::registry()
                    .with(filter)
                    .with(fmt_layer)
                    .with(tracing)
                    .with(metrics)
                    .try_init()
                    .expect("Failed initializing logger and tracing subscriber.");
            });
            return;
        }
    }

    with_format_layer!(&config.log_format, writer, |fmt_layer| {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .try_init()
            .expect("Failed initializing logger subscriber.");
    });
}

/// Initialize logging without OpenTelemetry.
pub fn init_logging(logging_config: Option<LoggingConfig>) {
    init(logging_config.unwrap_or_default(), false);
}

/// Initialize the runtime subscriber with logging and OpenTelemetry.
pub fn init_logging_and_tracing(logging_config: Option<LoggingConfig>) {
    init(logging_config.unwrap_or_default(), true);
}

/// Flush and shut down the global tracer provider.
pub fn flush_tracing() {
    if let Some(provider) = TRACER_PROVIDER.get() {
        let _ = provider.force_flush();
    }
}

