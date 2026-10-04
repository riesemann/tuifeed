//! tuifeed — RSS feeds and their articles in a terminal UI.
//!
//! The library holds the state, configuration and rendering; the `tuifeed`
//! binary wires them to a real terminal.

pub mod app;
pub mod config;
pub mod feed;
pub mod theme;
pub mod ui;
