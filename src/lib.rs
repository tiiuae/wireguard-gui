/// Parser and structure that defines accepted Wireguard configuration
/// file format.
pub mod config;
/// Component that helps display and update structure fields.
pub mod fields;
/// Settings that will be used during generation of configurations.
pub mod generation_settings;
/// Generator component. Provides functionality similar to https://www.wireguardconfig.com/
pub mod generator;
/// Overview of tunnel configuration.
pub mod overview;
/// Peers factory.
pub mod peer;
/// Tunnel - list item.
pub mod tunnel;
/// Various utility functions
pub mod utils;
