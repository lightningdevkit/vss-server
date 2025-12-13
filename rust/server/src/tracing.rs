use std::collections::HashMap;

use opentelemetry::propagation::Extractor;
use opentelemetry::propagation::TextMapPropagator;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::{SpanExporter as OtlpExporter, WithExportConfig};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::{trace::SdkTracerProvider, Resource};
use opentelemetry_stdout::SpanExporter as StdoutExporter;

use tracing::level_filters::LevelFilter;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{filter::Targets, fmt, layer::SubscriberExt};

pub struct HeaderExtractor<'a>(&'a HashMap<String, String>);

impl Extractor for HeaderExtractor<'_> {
	fn get(&self, key: &str) -> Option<&str> {
		self.0.get(key).map(|s| s.as_str())
	}

	fn keys(&self) -> Vec<&str> {
		self.0.keys().map(|k| k.as_str()).collect()
	}
}

pub fn extract_context(headers: &HashMap<String, String>) -> opentelemetry::Context {
	let propagator = TraceContextPropagator::new();
	propagator.extract(&HeaderExtractor(headers))
}

/// Initialize tracing subscriber for Jaeger and `stdout` backends.
pub fn configure_tracer() {
	let otlp_jaeger_exporter = OtlpExporter::builder()
		.with_tonic()
		.with_endpoint("http://localhost:4317")
		.build()
		.expect("Failed to create OTLP exporter");
	let stdout_exporter = StdoutExporter::default();

	let tracer_provider = SdkTracerProvider::builder()
		.with_batch_exporter(otlp_jaeger_exporter)
		.with_batch_exporter(stdout_exporter)
		.with_resource(Resource::builder().with_service_name("vss_server").build())
		.build();

	let tracer = tracer_provider.tracer("vss_server");

	tracing_subscriber::registry()
		.with(
			Targets::new()
				.with_default(LevelFilter::WARN)
				.with_target("vss_server", LevelFilter::INFO),
		)
		.with(fmt::layer().json())
		.with(OpenTelemetryLayer::new(tracer))
		.init();
}
