//! Andromeda core: logic without UI. Everything here is testable without opening a window.
pub mod auth;
pub mod instances;
pub mod launch;
pub mod session_store;
pub mod settings;
pub mod vanilla;

pub mod fabric;
pub mod fabric_install;
pub mod pkce;
pub mod quilt;
pub mod quilt_install;
pub mod upstream_install;
