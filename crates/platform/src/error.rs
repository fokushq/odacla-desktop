//! Platform error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("Failed to get foreground window: {0}")]
    ForegroundWindow(String),

    #[error("Failed to get window title: {0}")]
    WindowTitle(String),

    #[error("Failed to get process information: {0}")]
    ProcessInfo(String),

    #[error("Failed to get idle time: {0}")]
    IdleTime(String),

    #[error("Platform API error: {0}")]
    Api(String),
}
