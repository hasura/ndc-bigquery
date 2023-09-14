//! Metrics setup and update for our connector.

use std::time::Duration;

use prometheus::{Gauge, IntCounter, IntGauge, Registry};

use ndc_sdk::connector;

use super::configuration::InitializationError;

/// The collection of all metrics exposed through the `/metrics` endpoint.
#[derive(Debug, Clone)]
pub struct Metrics {
    query_total: IntCounter,
    explain_total: IntCounter,
    pool_size: IntGauge,
    pool_idle_count: IntGauge,
    pool_active_count: IntGauge,
    pool_max_connections: IntGauge,
    pool_min_connections: IntGauge,
    pool_acquire_timeout: Gauge,
    pool_max_lifetime: Gauge,
    pool_idle_timeout: Gauge,
}

impl Metrics {
    /// Set up counters and gauges used to produce Prometheus metrics
    pub fn initialize(
        metrics_registry: &mut Registry,
    ) -> Result<Self, connector::InitializationError> {
        let query_total = add_int_counter_metric(
            metrics_registry,
            "postgres_ndc_query_total",
            "Total successful queries.",
        )?;

        let explain_total = add_int_counter_metric(
            metrics_registry,
            "postgres_ndc_explain_total",
            "Total successful explains.",
        )?;

        let pool_size = add_int_gauge_metric(
            metrics_registry,
            "postgres_ndc_pool_size",
            "The number of connections currently active. This includes idle connections.",
        )?;

        let pool_idle_count = add_int_gauge_metric(
            metrics_registry,
            "postgres_ndc_pool_idle",
            "The number of connections active and idle (not in use).",
        )?;

        let pool_active_count = add_int_gauge_metric(
            metrics_registry,
            "postgres_ndc_pool_active",
            "The number of connections current active. This does not include idle connections.",
        )?;

        let pool_max_connections = add_int_gauge_metric(
            metrics_registry,
            "postgres_ndc_pool_max_connections",
            "The maximum number of connections that this pool should maintain.",
        )?;

        let pool_min_connections = add_int_gauge_metric(
            metrics_registry,
            "postgres_ndc_pool_min_connections",
            "The minimum number of connections that this pool should maintain.",
        )?;

        let pool_acquire_timeout = add_gauge_metric(
            metrics_registry,
            "postgres_ndc_pool_acquire_timeout",
            "Get the maximum amount of time to spend waiting for a connection, in seconds.",
        )?;

        let pool_idle_timeout = add_gauge_metric(
            metrics_registry,
            "postgres_ndc_pool_idle_timeout",
            "Get the maximum idle duration for individual connections, in seconds.",
        )?;

        let pool_max_lifetime = add_gauge_metric(
            metrics_registry,
            "postgres_ndc_pool_max_lifetime",
            "Get the maximum lifetime of individual connections, in seconds.",
        )?;

        Ok(Self {
            query_total,
            explain_total,
            pool_size,
            pool_idle_count,
            pool_active_count,
            pool_max_connections,
            pool_min_connections,
            pool_acquire_timeout,
            pool_idle_timeout,
            pool_max_lifetime,
        })
    }

    pub fn record_successful_query(&self) {
        self.query_total.inc()
    }

    pub fn record_successful_explain(&self) {
        self.explain_total.inc()
    }

    // Set the metrics populated from the pool options.
    //
    // This only needs to be called once, as the options don't change.
    pub fn set_pool_options_metrics(&self, pool_options: &sqlx::pool::PoolOptions<sqlx::Postgres>) {
        let max_connections: i64 = pool_options.get_max_connections().into();
        self.pool_max_connections.set(max_connections);

        let min_connections: i64 = pool_options.get_min_connections().into();
        self.pool_min_connections.set(min_connections);

        let acquire_timeout: f64 = pool_options.get_acquire_timeout().as_secs_f64();
        self.pool_acquire_timeout.set(acquire_timeout);

        // if nothing is set, return 0
        let idle_timeout: f64 = pool_options
            .get_idle_timeout()
            .unwrap_or(Duration::ZERO)
            .as_secs_f64();
        self.pool_idle_timeout.set(idle_timeout);

        // if nothing is set, return 0
        let max_lifetime: f64 = pool_options
            .get_max_lifetime()
            .unwrap_or(Duration::ZERO)
            .as_secs_f64();
        self.pool_max_lifetime.set(max_lifetime);
    }

    // Update all metrics fed from the database pool.
    pub fn update_pool_metrics(&self, pool: &sqlx::PgPool) {
        let pool_size: i64 = pool.size().into();
        self.pool_size.set(pool_size);

        let pool_idle: i64 = pool.num_idle().try_into().unwrap();
        self.pool_idle_count.set(pool_idle);

        let pool_active: i64 = pool_size - pool_idle;
        self.pool_active_count.set(pool_active);
    }
}

/// Create a new int counter metric and register it with the provided Prometheus Registry
fn add_int_counter_metric(
    metrics_registry: &mut Registry,
    metric_name: &str,
    metric_description: &str,
) -> Result<IntCounter, connector::InitializationError> {
    let int_counter = IntCounter::with_opts(prometheus::Opts::new(metric_name, metric_description))
        .map_err(wrap_prometheus_error)?;
    register_collector(metrics_registry, int_counter)
}

/// Create a new int gauge metric and register it with the provided Prometheus Registry
fn add_int_gauge_metric(
    metrics_registry: &mut Registry,
    metric_name: &str,
    metric_description: &str,
) -> Result<IntGauge, connector::InitializationError> {
    let int_gauge = IntGauge::with_opts(prometheus::Opts::new(metric_name, metric_description))
        .map_err(wrap_prometheus_error)?;
    register_collector(metrics_registry, int_gauge)
}

/// Create a new gauge metric and register it with the provided Prometheus Registry
fn add_gauge_metric(
    metrics_registry: &mut Registry,
    metric_name: &str,
    metric_description: &str,
) -> Result<Gauge, connector::InitializationError> {
    let gauge = Gauge::with_opts(prometheus::Opts::new(metric_name, metric_description))
        .map_err(wrap_prometheus_error)?;
    register_collector(metrics_registry, gauge)
}

/// Register a new collector with the registry, and returns it for later use.
fn register_collector<Collector: prometheus::core::Collector + std::clone::Clone + 'static>(
    metrics_registry: &mut Registry,
    collector: Collector,
) -> Result<Collector, connector::InitializationError> {
    metrics_registry
        .register(Box::new(collector.clone()))
        .map_err(wrap_prometheus_error)?;
    Ok(collector)
}

fn wrap_prometheus_error(err: prometheus::Error) -> connector::InitializationError {
    connector::InitializationError::Other(InitializationError::PrometheusError(err).into())
}
