//! Subcommand handlers — each one wires a clap subcommand to the
//! corresponding library primitive.
pub mod audit;
pub mod costs;
pub mod cron;
pub mod db;
pub mod doctor;
pub mod mcp;
pub mod memory;
pub mod provider;
pub mod route;
pub mod session;
pub mod skill;
pub mod tui;
pub mod workspace;
