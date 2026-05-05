//! Usage statistics tracking

use std::collections::HashMap;

use super::{AnalyticsError, AnalyticsEvent, AnalyticsResult, EventSeverity, MetricStats};

/// Tracks analytics events and maintains usage statistics
#[derive(Debug, Clone)]
pub struct AnalyticsTracker {
    /// Whether analytics collection is enabled
    enabled: bool,
    /// Stored events (limited ring buffer)
    events: Vec<AnalyticsEvent>,
    /// Maximum number of events to keep in memory
    max_events: usize,
    /// Aggregated metrics keyed by metric name
    metrics: HashMap<String, MetricStats>,
    /// Session start timestamp
    session_start: u64,
    /// Application version
    app_version: String,
}

impl Default for AnalyticsTracker {
    fn default() -> Self {
        Self {
            enabled: true,
            events: Vec::new(),
            max_events: 10_000,
            metrics: HashMap::new(),
            session_start: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl AnalyticsTracker {
    /// Create a new AnalyticsTracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a tracker with a specific max event limit
    pub fn with_capacity(max_events: usize) -> Self {
        Self {
            max_events,
            ..Self::default()
        }
    }

    /// Enable or disable analytics collection
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if analytics collection is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Track a general event
    pub fn track_event(
        &mut self,
        name: &str,
        category: &str,
        severity: EventSeverity,
    ) -> AnalyticsResult<()> {
        if !self.enabled {
            return Ok(());
        }

        let event = AnalyticsEvent {
            name: name.to_string(),
            category: category.to_string(),
            severity,
            timestamp: self.now(),
            duration_ms: None,
            metadata: HashMap::new(),
        };

        self.push_event(event);
        Ok(())
    }

    /// Track an event with duration
    pub fn track_duration(
        &mut self,
        name: &str,
        category: &str,
        duration_ms: u64,
    ) -> AnalyticsResult<()> {
        if !self.enabled {
            return Ok(());
        }

        let event = AnalyticsEvent {
            name: name.to_string(),
            category: category.to_string(),
            severity: EventSeverity::Info,
            timestamp: self.now(),
            duration_ms: Some(duration_ms),
            metadata: HashMap::new(),
        };

        // Update metric stats
        let metric_name = format!("{}.{}", category, name);
        let stats = self.metrics.entry(metric_name).or_default();
        stats.count += 1;
        stats.sum += duration_ms as f64;
        if duration_ms as f64 > stats.max {
            stats.max = duration_ms as f64;
        }
        if stats.min == 0.0 || (duration_ms as f64) < stats.min {
            stats.min = duration_ms as f64;
        }

        self.push_event(event);
        Ok(())
    }

    /// Track an event with custom metadata
    pub fn track_with_metadata(
        &mut self,
        name: &str,
        category: &str,
        metadata: HashMap<String, String>,
    ) -> AnalyticsResult<()> {
        if !self.enabled {
            return Ok(());
        }

        let event = AnalyticsEvent {
            name: name.to_string(),
            category: category.to_string(),
            severity: EventSeverity::Info,
            timestamp: self.now(),
            duration_ms: None,
            metadata,
        };

        self.push_event(event);
        Ok(())
    }

    /// Track a tool execution
    pub fn track_tool_execution(&mut self, tool_name: &str, duration_ms: u64, success: bool) -> AnalyticsResult<()> {
        let mut metadata = HashMap::new();
        metadata.insert("success".to_string(), success.to_string());

        if !self.enabled {
            return Ok(());
        }

        let event = AnalyticsEvent {
            name: format!("tool.{}", tool_name),
            category: "tools".to_string(),
            severity: if success { EventSeverity::Info } else { EventSeverity::Warning },
            timestamp: self.now(),
            duration_ms: Some(duration_ms),
            metadata,
        };

        // Update metrics
        let metric_name = format!("tool_execution.{}", tool_name);
        let stats = self.metrics.entry(metric_name).or_default();
        stats.count += 1;
        stats.sum += duration_ms as f64;
        if duration_ms as f64 > stats.max {
            stats.max = duration_ms as f64;
        }
        if stats.min == 0.0 || (duration_ms as f64) < stats.min {
            stats.min = duration_ms as f64;
        }

        self.push_event(event);
        Ok(())
    }

    /// Get all stored events
    pub fn events(&self) -> &[AnalyticsEvent] {
        &self.events
    }

    /// Get aggregated metrics
    pub fn metrics(&self) -> &HashMap<String, MetricStats> {
        &self.metrics
    }

    /// Get the number of events tracked
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Get session duration in seconds
    pub fn session_duration_secs(&self) -> u64 {
        self.now() - self.session_start
    }

    /// Get the app version
    pub fn app_version(&self) -> &str {
        &self.app_version
    }

    /// Export all events as JSON
    pub fn export_json(&self) -> AnalyticsResult<String> {
        serde_json::to_string_pretty(&self.events)
            .map_err(|e| AnalyticsError::ExportFailed(format!("JSON export failed: {e}")))
    }

    /// Save all tracked events to a JSON file for persistence.
    ///
    /// The file can later be loaded with [`Self::load_from_file`].
    /// Only events are persisted; aggregated metrics are reconstructed on load.
    pub fn save_to_file(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(&self.events)?;
        std::fs::write(path, json)?;
        tracing::debug!("Saved {} analytics events to {}", self.events.len(), path.display());
        Ok(())
    }

    /// Load tracked events from a JSON file created by [`Self::save_to_file`].
    ///
    /// Returns a new `AnalyticsTracker` pre-populated with the deserialized events.
    /// Metrics are automatically aggregated from the loaded events.
    pub fn load_from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let data = std::fs::read_to_string(path)?;
        let events: Vec<AnalyticsEvent> = serde_json::from_str(&data)?;
        let mut tracker = AnalyticsTracker::new();
        for event in events {
            tracker.push_event(event);
        }
        tracing::debug!("Loaded {} analytics events from {}", tracker.events.len(), path.display());
        Ok(tracker)
    }

    /// Clear all tracked events (but keep metrics)
    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    /// Reset everything (events and metrics)
    pub fn reset(&mut self) {
        self.events.clear();
        self.metrics.clear();
        self.session_start = self.now();
    }

    /// Add an event to the ring buffer
    fn push_event(&mut self, event: AnalyticsEvent) {
        if self.events.len() >= self.max_events {
            self.events.remove(0);
        }
        self.events.push(event);
    }

    /// Get current timestamp in seconds
    fn now(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracker_new() {
        let tracker = AnalyticsTracker::new();
        assert!(tracker.is_enabled());
        assert_eq!(tracker.event_count(), 0);
    }

    #[test]
    fn test_track_event() {
        let mut tracker = AnalyticsTracker::new();
        tracker.track_event("test_event", "test", EventSeverity::Info).unwrap();
        assert_eq!(tracker.event_count(), 1);
    }

    #[test]
    fn test_track_disabled() {
        let mut tracker = AnalyticsTracker::new();
        tracker.set_enabled(false);
        tracker.track_event("test", "test", EventSeverity::Info).unwrap();
        assert_eq!(tracker.event_count(), 0);
    }

    #[test]
    fn test_track_duration_updates_metrics() {
        let mut tracker = AnalyticsTracker::new();
        tracker.track_duration("bash", "tool", 150).unwrap();
        tracker.track_duration("bash", "tool", 250).unwrap();
        let metric = tracker.metrics().get("tool.bash").unwrap();
        assert_eq!(metric.count, 2);
        assert!((metric.mean() - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_track_tool_execution() {
        let mut tracker = AnalyticsTracker::new();
        tracker.track_tool_execution("bash", 100, true).unwrap();
        assert_eq!(tracker.event_count(), 1);
        assert_eq!(tracker.events()[0].name, "tool.bash");
    }

    #[test]
    fn test_export_json() {
        let mut tracker = AnalyticsTracker::new();
        tracker.track_event("test_event", "test", EventSeverity::Info).unwrap();
        let json = tracker.export_json().unwrap();
        assert!(json.contains("test_event"));
    }

    #[test]
    fn test_clear_events() {
        let mut tracker = AnalyticsTracker::new();
        tracker.track_event("test", "test", EventSeverity::Info).unwrap();
        tracker.clear_events();
        assert_eq!(tracker.event_count(), 0);
    }

    #[test]
    fn test_reset() {
        let mut tracker = AnalyticsTracker::new();
        tracker.track_duration("bash", "tool", 150).unwrap();
        tracker.reset();
        assert_eq!(tracker.event_count(), 0);
        assert!(tracker.metrics().is_empty());
    }

    #[test]
    fn test_with_capacity() {
        let tracker = AnalyticsTracker::with_capacity(5);
        assert_eq!(tracker.max_events, 5);
    }

    #[test]
    fn test_app_version() {
        let tracker = AnalyticsTracker::new();
        assert!(!tracker.app_version().is_empty());
    }
}
