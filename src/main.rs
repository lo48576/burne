//! burne: BUlk ReName by Editor.
#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]
// `clippy::missing_docs_in_private_items` implies `missing_docs`.
#![warn(clippy::missing_docs_in_private_items)]
#![warn(clippy::unwrap_used)]

/// Entrypoint.
fn main() {
    init_logger();
}

/// Initialize logger.
fn init_logger() {
    /// Default log filter for debug build.
    #[cfg(debug_assertions)]
    const DEFAULT_LOG_FILTER: &str = "burne=debug";
    /// Default log filter for release build.
    #[cfg(not(debug_assertions))]
    const DEFAULT_LOG_FILTER: &str = "burne=warn";

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(DEFAULT_LOG_FILTER))
        .init();
}
