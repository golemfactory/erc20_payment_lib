use lazy_static::lazy_static;
use metrics_core::{Builder, Drain, Observe};
use metrics_runtime::observers::PrometheusBuilder;

use metrics_runtime::Controller;
use std::error::Error;
use std::sync::Arc;
use std::sync::Mutex;

lazy_static! {
    static ref METRICS: Arc<Mutex<Option<Metrics>>> = Arc::new(Mutex::new(None));
}
/// Exports metrics by converting them to a textual representation and logging them.
pub struct StringExporter {
    controller: Controller,
    builder: PrometheusBuilder,
}

impl StringExporter {
    /// Creates a new [`StringExporter`] that logs at the configurable level.
    ///
    /// Observers expose their output by being converted into strings.
    pub fn new(controller: Controller, builder: PrometheusBuilder) -> Self {
        StringExporter {
            controller,
            builder,
        }
    }

    /// Run this exporter, logging output only once.
    pub fn turn(&mut self) -> String {
        let mut observer = self.builder.build();
        self.controller.observe(&mut observer);
        observer.drain()
    }
}

struct Metrics {
    pub exporter: StringExporter,
}

impl Metrics {
    fn new() -> Metrics {
        let receiver = metrics_runtime::Receiver::builder()
            .build()
            .expect("Metrics initialization failure");
        let exporter = StringExporter::new(
            receiver.controller(),
            PrometheusBuilder::new().set_quantiles(&[
                0.0, 0.01, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.95, 0.99, 0.999,
            ]),
        );
        receiver.install();

        Self { exporter }
    }

    pub fn export(&mut self) -> String {
        self.exporter.turn()
    }
}

pub fn init_metrics() {
    let mut lock = METRICS.lock().expect("Failed to lock metrics");
    match lock.as_mut() {
        Some(_) => {
            log::warn!("Metrics already initialized - skipping initialization");
            eprintln!("WARN - Metrics already initialized - skipping initialization");
        }
        None => {
            *lock = Some(Metrics::new());
        }
    }
}

//algorith is returning metrics in random order, which is fine for prometheus, but not for human checking metrics
pub fn sort_metrics_txt(metrics: &str) -> String {
    let Some(first_line_idx) = metrics.find('\n') else {
        return metrics.to_string();
    };
    let (first_line, metrics_content) = metrics.split_at(first_line_idx);

    let mut entries = metrics_content
        .split("\n\n") //splitting by double new line to get separate metrics
        .map(|s| {
            let trimmed = s.trim();
            let mut lines = trimmed.split('\n').collect::<Vec<_>>();
            lines.sort(); //sort by properties
            lines.join("\n")
        })
        .collect::<Vec<String>>();
    entries.sort(); //sort by metric name

    first_line.to_string() + "\n" + entries.join("\n\n").as_str()
}

pub fn export_metrics_to_prometheus() -> Result<String, Box<dyn Error>> {
    let mut lock = METRICS.lock().expect("Failed to lock metrics");
    match lock.as_mut() {
        Some(metrics) => Ok(sort_metrics_txt(&metrics.export())),
        None => Err("Metric exporter uninitialized".into()),
    }
}
