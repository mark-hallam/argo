//! Subcommand handlers — each one wires a clap subcommand to the
//! corresponding library primitive.
pub mod agents;
pub mod audit;
pub mod costs;
pub mod cron;
pub mod db;
pub mod doctor;
pub mod eval;
pub mod forge;
pub mod gateway;
pub mod judge;
pub mod mcp;
pub mod memory;
pub mod model;
pub mod plugin;
pub mod provider;
pub mod route;
pub mod session;
pub mod skill;
pub mod trace;
pub mod tui;
pub mod workspace;
