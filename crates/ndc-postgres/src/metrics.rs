//! Metrics setup and update for our connector.

use super::configuration::InitializationError;
use ndc_hub::connector;
use prometheus::core::{AtomicF64, AtomicI64, AtomicU64, GenericCounter, GenericGauge};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Metrics {
    pub query_total: GenericCounter<AtomicU64>,
    pub explain_total: GenericCounter<AtomicU64>,
    pub pool_size: GenericGauge<AtomicI64>,
    pub pool_idle_count: GenericGauge<AtomicI64>,
    pub pool_active_count: GenericGauge<AtomicI64>,
    pub pool_max_connections: GenericGauge<AtomicI64>,
    pub pool_min_connections: GenericGauge<AtomicI64>,
    pub pool_acquire_timeout: GenericGauge<AtomicF64>,
    pub pool_max_lifetime: GenericGauge<AtomicF64>,
    pub pool_idle_timeout: GenericGauge<AtomicF64>,
}

/// Create a new int counter metric and register it with the provided Prometheus Registry
fn add_int_counter_metric(
    metrics_registry: &mut prometheus::Registry,
    metric_name: &str,
    metric_description: &str,
) -> Result<GenericCounter<AtomicU64>, connector::InitializationError> {
    let int_counter =
        prometheus::IntCounter::with_opts(prometheus::Opts::new(metric_name, metric_description))
            .map_err(|prometheus_error| {
            connector::InitializationError::Other(
                InitializationError::PrometheusError(prometheus_error).into(),
            )
        })?;

    metrics_registry
        .register(Box::new(int_counter.clone()))
        .map_err(|prometheus_error| {
            connector::InitializationError::Other(
                InitializationError::PrometheusError(prometheus_error).into(),
            )
        })?;

    Ok(int_counter)
}

/// Create a new int gauge metric and register it with the provided Prometheus Registry
fn add_int_gauge_metric(
    metrics_registry: &mut prometheus::Registry,
    metric_name: &str,
    metric_description: &str,
) -> Result<GenericGauge<AtomicI64>, connector::InitializationError> {
    let int_gauge =
        prometheus::IntGauge::with_opts(prometheus::Opts::new(metric_name, metric_description))
            .map_err(|prometheus_error| {
                connector::InitializationError::Other(
                    InitializationError::PrometheusError(prometheus_error).into(),
                )
            })?;

    metrics_registry
        .register(Box::new(int_gauge.clone()))
        .map_err(|prometheus_error| {
            connector::InitializationError::Other(
                InitializationError::PrometheusError(prometheus_error).into(),
            )
        })?;

    Ok(int_gauge)
}

/// Create a new gauge metric and register it with the provided Prometheus Registry
fn add_gauge_metric(
    metrics_registry: &mut prometheus::Registry,
    metric_name: &str,
    metric_description: &str,
) -> Result<GenericGauge<AtomicF64>, connector::InitializationError> {
    let gauge =
        prometheus::Gauge::with_opts(prometheus::Opts::new(metric_name, metric_description))
            .map_err(|prometheus_error| {
                connector::InitializationError::Other(
                    InitializationError::PrometheusError(prometheus_error).into(),
                )
            })?;

    metrics_registry
        .register(Box::new(gauge.clone()))
        .map_err(|prometheus_error| {
            connector::InitializationError::Other(
                InitializationError::PrometheusError(prometheus_error).into(),
            )
        })?;

    Ok(gauge)
}

/// Setup counters and gauges used to produce Prometheus metrics
pub async fn initialise_metrics(
    metrics_registry: &mut prometheus::Registry,
) -> Result<Metrics, connector::InitializationError> {
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

    Ok(Metrics {
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

// update all Prometheus counters
pub fn update_pool_metrics(pool: &sqlx::PgPool, metrics: &Metrics) {
    let pool_size: i64 = pool.size().into();
    metrics.pool_size.set(pool_size);

    let pool_idle: i64 = pool.num_idle().try_into().unwrap();
    metrics.pool_idle_count.set(pool_idle);

    let pool_active: i64 = pool_size - pool_idle;
    metrics.pool_active_count.set(pool_active);

    let pool_options = pool.options();

    let max_connections: i64 = pool_options.get_max_connections().into();
    metrics.pool_max_connections.set(max_connections);

    let min_connections: i64 = pool_options.get_min_connections().into();
    metrics.pool_min_connections.set(min_connections);

    let acquire_timeout: f64 = pool_options.get_acquire_timeout().as_secs_f64();
    metrics.pool_acquire_timeout.set(acquire_timeout);

    // if nothing is set, return 0
    let idle_timeout: f64 = pool_options
        .get_idle_timeout()
        .unwrap_or(Duration::ZERO)
        .as_secs_f64();
    metrics.pool_idle_timeout.set(idle_timeout);

    // if nothing is set, return 0
    let max_lifetime: f64 = pool_options
        .get_max_lifetime()
        .unwrap_or(Duration::ZERO)
        .as_secs_f64();
    metrics.pool_max_lifetime.set(max_lifetime);
}
