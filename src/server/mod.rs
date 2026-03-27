//! Server module

pub mod bypass_exec;
pub mod bypass_session;
pub mod daemon;
pub mod interaction_detector;
pub mod pty;
pub mod screen_buffer;
pub mod session;
pub mod status_file;
pub mod strategy;

pub use bypass_exec::{execute_bypass_command, is_process_running};
pub use bypass_session::BypassSession;
pub use daemon::{Daemon, DaemonConfig};
pub use interaction_detector::InteractionDetector;
pub use pty::{Pty, PtySize};
pub use screen_buffer::ScreenBuffer;
pub use session::{Session, SessionEvent, SessionHandle};
pub use status_file::{BypassStatus, StatusFile};
pub use strategy::{Strategy, StrategyEngine};
