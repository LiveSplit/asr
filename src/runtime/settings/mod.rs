//! Support for interacting with the settings of the auto splitter.
//!
//! # Overview
//!
//! Settings consist of two parts. One part is the settings [`Gui`], that is
//! used to let the user configure the settings. The other part is the settings
//! values that are actually stored in the splits file. Those settings don't
//! necessarily correlate entirely with the settings [`Gui`], because the stored
//! splits might either be from a different version of the auto splitter or
//! contain additional information such as the version of the settings, that the
//! user doesn't necessarily directly interact with. These stored settings are
//! available as the global settings [`Map`], which can be loaded, modified and
//! stored freely. The keys used for the settings widgets directly correlate
//! with the keys used in the settings [`Map`]. Any changes in the settings
//! [`Gui`] will automatically be reflected in the global settings [`Map`] and
//! vice versa.
//!
//! # Defining a GUI
//!
//! ```ignore
//! #[derive(Gui)]
//! struct Settings {
//!     /// General Settings
//!     _general_settings: Title,
//!     /// Use Game Time
//!     ///
//!     /// This is the tooltip.
//!     use_game_time: bool,
//! }
//! ```
//!
//! The type can then be used like so:
//!
//! ```ignore
//! let mut settings = Settings::register();
//!
//! loop {
//!    settings.update();
//!    // Do something with the settings.
//! }
//! ```
//!
//! Check the [`Gui`](macro@Gui) derive macro and the [`Gui`](trait@Gui) trait
//! for more information.
//!
//! # Modifying the global settings map
//!
//! ```no_run
//! # use asr::settings;
//! let mut map = settings::Map::load();
//! map.insert("key", true);
//! map.store();
//! ```
//!
//! Check the [`Map`](struct@Map) struct for more information.

pub mod gui;
mod list;
mod map;
mod value;

pub use gui::Gui;
pub use list::*;
pub use map::*;
pub use value::*;
