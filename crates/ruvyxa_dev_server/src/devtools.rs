//! Development observability for routes, bundles, server actions, and the render cache.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::render_cache::RenderCacheSnapshot;

#[derive(Debug)]
pub(crate) struct DevToolsMetrics {
    started: Instant,
    inner: Mutex<MetricState>,
}

#[derive(Debug, Default)]
struct MetricState {
    bundles: BTreeMap<String, BundleMetric>,
    actions: BTreeMap<String, ActionMetric>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BundleMetric {
    path: String,
    bytes: usize,
    builds: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActionMetric {
    action: String,
    calls: u64,
    errors: u64,
    total_micros: u128,
    last_micros: u128,
    max_micros: u128,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DevToolsSnapshot {
    uptime_seconds: u64,
    routes: serde_json::Value,
    cache: RenderCacheSnapshot,
    bundles: Vec<BundleMetric>,
    actions: Vec<ActionMetric>,
}

impl Default for DevToolsMetrics {
    fn default() -> Self {
        Self {
            started: Instant::now(),
            inner: Mutex::new(MetricState::default()),
        }
    }
}

impl DevToolsMetrics {
    pub(crate) fn record_bundle(&self, path: &str, bytes: usize) {
        let Ok(mut state) = self.inner.lock() else {
            return;
        };
        let metric = state
            .bundles
            .entry(path.to_string())
            .or_insert_with(|| BundleMetric {
                path: path.to_string(),
                bytes,
                builds: 0,
            });
        metric.bytes = bytes;
        metric.builds = metric.builds.saturating_add(1);
    }

    pub(crate) fn record_action(&self, path: &str, action: &str, elapsed: Duration, error: bool) {
        let Ok(mut state) = self.inner.lock() else {
            return;
        };
        let key = format!("{path}#{action}");
        let metric = state
            .actions
            .entry(key.clone())
            .or_insert_with(|| ActionMetric {
                action: key,
                ..ActionMetric::default()
            });
        let micros = elapsed.as_micros();
        metric.calls = metric.calls.saturating_add(1);
        metric.errors = metric.errors.saturating_add(u64::from(error));
        metric.total_micros = metric.total_micros.saturating_add(micros);
        metric.last_micros = micros;
        metric.max_micros = metric.max_micros.max(micros);
    }

    pub(crate) fn snapshot(
        &self,
        routes: serde_json::Value,
        cache: RenderCacheSnapshot,
    ) -> DevToolsSnapshot {
        let (bundles, actions) = self.inner.lock().map_or_else(
            |_| (Vec::new(), Vec::new()),
            |state| {
                (
                    state.bundles.values().cloned().collect(),
                    state.actions.values().cloned().collect(),
                )
            },
        );
        DevToolsSnapshot {
            uptime_seconds: self.started.elapsed().as_secs(),
            routes,
            cache,
            bundles,
            actions,
        }
    }
}

pub(crate) fn dashboard_html() -> &'static str {
    include_str!("../templates/devtools.html")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn metrics_aggregate_bundle_builds_and_action_timings() {
        let metrics = DevToolsMetrics::default();
        metrics.record_bundle("/", 10);
        metrics.record_bundle("/", 14);
        metrics.record_action("/todos", "save", Duration::from_millis(2), false);
        metrics.record_action("/todos", "save", Duration::from_millis(5), true);
        let cache = crate::RenderCache::new(2, 60).snapshot().await;
        let snapshot = metrics.snapshot(serde_json::json!([]), cache);
        assert_eq!(snapshot.bundles[0].bytes, 14);
        assert_eq!(snapshot.bundles[0].builds, 2);
        assert_eq!(snapshot.actions[0].calls, 2);
        assert_eq!(snapshot.actions[0].errors, 1);
        assert_eq!(snapshot.actions[0].max_micros, 5_000);
    }

    #[test]
    fn dashboard_is_self_contained_and_accessible() {
        let html = dashboard_html();
        assert!(html.contains("Ruvyxa DevTools"));
        assert!(html.contains("aria-label=\"Filter routes\""));
        assert!(html.contains("prefers-reduced-motion"));
    }
}
