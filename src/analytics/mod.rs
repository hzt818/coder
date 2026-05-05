//! Analytics module - usage statistics and telemetry
//!
//! Provides event tracking, usage statistics collection,
//! and aggregate reporting for understanding how the tool is used.

pub mod tracker;

pub use tracker::AnalyticsTracker;

/// Result type for analytics operations
pub type AnalyticsResult<T> = std::result::Result<T, AnalyticsError>;

/// Errors that can occur during analytics operations
#[derive(Debug, thiserror::Error)]
pub enum AnalyticsError {
    /// Storage error
    #[error("Analytics storage error: {0}")]
    Storage(String),
    /// Serialization error
    #[error("Analytics serialization error: {0}")]
    Serialization(String),
    /// Tracker not initialized
    #[error("Analytics tracker not initialized")]
    NotInitialized,
    /// Export failed
    #[error("Analytics export failed: {0}")]
    ExportFailed(String),
}

/// Severity level of an analytics event
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EventSeverity {
    /// Debug / verbose event
    Debug,
    /// Informational event
    Info,
    /// Warning event
    Warning,
    /// Error event
    Error,
    /// Critical event
    Critical,
}

impl EventSeverity {
    /// Return the severity as a string label
    pub fn as_str(&self) -> &'static str {
        match self {
            EventSeverity::Debug => "debug",
            EventSeverity::Info => "info",
            EventSeverity::Warning => "warning",
            EventSeverity::Error => "error",
            EventSeverity::Critical => "critical",
        }
    }
}

/// A single analytics event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnalyticsEvent {
    /// Event name
    pub name: String,
    /// Event category
    pub category: String,
    /// Event severity
    pub severity: EventSeverity,
    /// Timestamp (Unix epoch seconds)
    pub timestamp: u64,
    /// Optional duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Optional metadata key-value pairs
    pub metadata: std::collections::HashMap<String, String>,
}

/// Aggregate statistics for a specific metric
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MetricStats {
    /// Number of events
    pub count: u64,
    /// Minimum value
    pub min: f64,
    /// Maximum value
    pub max: f64,
    /// Sum of all values
    pub sum: f64,
}

impl MetricStats {
    /// Calculate the mean (average) value
    pub fn mean(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_severity_as_str() {
        assert_eq!(EventSeverity::Debug.as_str(), "debug");
        assert_eq!(EventSeverity::Info.as_str(), "info");
        assert_eq!(EventSeverity::Warning.as_str(), "warning");
        assert_eq!(EventSeverity::Error.as_str(), "error");
        assert_eq!(EventSeverity::Critical.as_str(), "critical");
    }

    #[test]
    fn test_analytics_error_display() {
        let err = AnalyticsError::Storage("disk full".to_string());
        assert_eq!(err.to_string(), "Analytics storage error: disk full");
    }

    #[test]
    fn test_metric_stats_mean() {
        let stats = MetricStats {
            count: 4,
            sum: 100.0,
            ..Default::default()
        };
        assert!((stats.mean() - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_metric_stats_mean_empty() {
        let stats = MetricStats::default();
        assert!((stats.mean() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_event_serialization() {
        let event = AnalyticsEvent {
            name: "tool_executed".to_string(),
            category: "tools".to_string(),
            severity: EventSeverity::Info,
            timestamp: 1_700_000_000,
            duration_ms: Some(150),
            metadata: std::collections::HashMap::new(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: AnalyticsEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "tool_executed");
        assert_eq!(deserialized.duration_ms, Some(150));
    }
}
