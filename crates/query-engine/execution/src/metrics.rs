//! Metrics setup and update for our connector.

use prometheus::{Histogram, HistogramTimer, IntCounter, Registry};

/// The collection of all metrics exposed through the `/metrics` endpoint.
#[derive(Debug, Clone)]
pub struct Metrics {
    query_total: IntCounter,
    explain_total: IntCounter,
    query_plan_time: Histogram,
    query_execution_time: Histogram,
    pub error_metrics: ErrorMetrics,
}

impl Metrics {
    /// Set up counters and gauges used to produce Prometheus metrics
    pub fn initialize(metrics_registry: &mut Registry) -> Result<Self, prometheus::Error> {
        let query_total = add_int_counter_metric(
            metrics_registry,
            "bigquery_ndc_query_total",
            "Total successful queries.",
        )?;

        let explain_total = add_int_counter_metric(
            metrics_registry,
            "bigquery_ndc_explain_total",
            "Total successful explains.",
        )?;

        let query_plan_time = add_histogram_metric(
            metrics_registry,
            "bigquery_ndc_query_plan_time",
            "Time taken to plan a query for execution, in seconds.",
        )?;

        let query_execution_time = add_histogram_metric(
            metrics_registry,
            "bigquery_ndc_query_execution_time",
            "Time taken to execute an already-planned query, in seconds.",
        )?;

        let error_metrics = ErrorMetrics::initialize(metrics_registry)?;

        Ok(Self {
            query_total,
            explain_total,
            query_plan_time,
            query_execution_time,
            error_metrics,
        })
    }

    pub fn record_successful_query(&self) {
        self.query_total.inc();
    }

    pub fn record_successful_explain(&self) {
        self.explain_total.inc();
    }

    pub fn time_query_plan(&self) -> Timer {
        Timer(self.query_plan_time.start_timer())
    }

    pub fn time_query_execution(&self) -> Timer {
        Timer(self.query_execution_time.start_timer())
    }
}

/// Create a new int counter metric and register it with the provided Prometheus Registry
fn add_int_counter_metric(
    metrics_registry: &mut Registry,
    metric_name: &str,
    metric_description: &str,
) -> Result<IntCounter, prometheus::Error> {
    let int_counter =
        IntCounter::with_opts(prometheus::Opts::new(metric_name, metric_description))?;
    register_collector(metrics_registry, int_counter)
}

/// Create a new histogram metric using the default buckets, and register it with the provided
/// Prometheus Registry.
fn add_histogram_metric(
    metrics_registry: &mut prometheus::Registry,
    metric_name: &str,
    metric_description: &str,
) -> Result<Histogram, prometheus::Error> {
    let histogram = Histogram::with_opts(prometheus::HistogramOpts::new(
        metric_name,
        metric_description,
    ))?;
    register_collector(metrics_registry, histogram)
}

/// Register a new collector with the registry, and returns it for later use.
fn register_collector<Collector: prometheus::core::Collector + std::clone::Clone + 'static>(
    metrics_registry: &mut Registry,
    collector: Collector,
) -> Result<Collector, prometheus::Error> {
    metrics_registry.register(Box::new(collector.clone()))?;
    Ok(collector)
}

/// A wrapper around the Prometheus [HistogramTimer] that can make a decision
/// on whether to record or not based on a result.
pub struct Timer(HistogramTimer);

impl Timer {
    /// Stops the timer, recording if the result is `Ok`, and discarding it if
    /// the result is an `Err`. It returns its input for convenience.
    pub fn complete_with<T, E>(self, result: Result<T, E>) -> Result<T, E> {
        match result {
            Ok(_) => {
                self.0.stop_and_record();
            }
            Err(_) => {
                self.0.stop_and_discard();
            }
        };
        result
    }
}

// /// A wrapper around the internal Prometheus error type to avoid exposing more
// /// than we need.
// #[derive(Debug)]
// pub struct Error(prometheus::Error);

// impl std::fmt::Display for Error {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         self.0.fmt(f)
//     }
// }

// impl std::error::Error for Error {}

/// A collection of metrics indicating errors.
#[derive(Debug, Clone)]
pub struct ErrorMetrics {
    /// the connector received an invalid request.
    invalid_request_total: IntCounter,
    /// the connector received a request using capabilities it does not support.
    unsupported_capability_total: IntCounter,
    /// the connector could not fulfill a request because it does not support
    /// certain features (which are not described as capabilities).
    unsupported_feature_total: IntCounter,
    /// the connector had an internal error.
    connector_error_total: IntCounter,
    /// the database emmited an error.
    database_error_total: IntCounter,
    /// we failed to acquire a database connection from the pool
    connection_acquisition_error_total: IntCounter,
}

impl ErrorMetrics {
    /// Set up counters and gauges used to produce Prometheus metrics
    pub fn initialize(
        metrics_registry: &mut prometheus::Registry,
    ) -> Result<Self, prometheus::Error> {
        let invalid_request_total = add_int_counter_metric(
            metrics_registry,
            "ndc_bigquery_error_invalid_request_total_count",
            "Total number of invalid requests encountered.",
        )?;

        let unsupported_capability_total = add_int_counter_metric(
            metrics_registry,
            "ndc_bigquery_error_unsupported_capability_total_count",
            "Total number of invalid requests with unsupported capabilities encountered.",
        )?;

        let unsupported_feature_total = add_int_counter_metric(
            metrics_registry,
            "ndc_bigquery_error_unsupported_capabilities_total_count",
            "Total number of invalid requests with unsupported capabilities encountered.",
        )?;

        let connector_error_total = add_int_counter_metric(
            metrics_registry,
            "ndc_bigquery_error_connector_error_total_count",
            "Total number of requests failed due to an internal conenctor error.",
        )?;

        let database_error_total = add_int_counter_metric(
            metrics_registry,
            "ndc_bigquery_error_database_error_total_count",
            "Total number of requests failed due to a database error.",
        )?;

        let connection_acquisition_error_total = add_int_counter_metric(
            metrics_registry,
            "ndc_bigquery_error_connection_acquisition_error_total_count",
            "Total number of failures to acquire a database connection.",
        )?;

        Ok(ErrorMetrics {
            invalid_request_total,
            unsupported_capability_total,
            unsupported_feature_total,
            connector_error_total,
            database_error_total,
            connection_acquisition_error_total,
        })
    }

    pub fn record_invalid_request(&self) {
        self.invalid_request_total.inc();
    }
    pub fn record_unsupported_capability(&self) {
        self.unsupported_capability_total.inc();
    }
    pub fn record_unsupported_feature(&self) {
        self.unsupported_feature_total.inc();
    }
    pub fn record_connector_error(&self) {
        self.connector_error_total.inc();
    }
    pub fn record_database_error(&self) {
        self.database_error_total.inc();
    }
    pub fn record_connection_acquisition_error(&self) {
        self.connection_acquisition_error_total.inc();
    }
}
