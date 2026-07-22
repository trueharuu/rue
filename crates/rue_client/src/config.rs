//! Configuration for the TETR.IO client, including game handling settings.
use triangle::types::game::Buffering;
use triangle::types::game::Handling;

/// Configuration for the TETR.IO client, including game handling settings.
pub struct Config {
    /// Game handling settings for the client.
    pub handling: Handling,
}

/// Default configuration for the client.
pub const CONFIG: Config = Config {
    handling: Handling {
        arr: 0.0,
        das: 5.0,
        ihs: Buffering::Tap,
        irs: Buffering::Tap,
        dcd: 0.0,
        sdf: 41.0,
        safelock: false,
        cancel: false,
        may20g: true,
    },
};
