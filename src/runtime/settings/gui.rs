//! This module allows you to add settings widgets to the settings GUI that the
//! user can modify.

#[cfg(feature = "derive")]
pub use asr_derive::Gui;

use crate::{runtime::sys, watcher::Pair};

use super::map::Map;

/// Adds a new boolean setting widget to the settings GUI that the user can
/// modify. This will return either the specified default value or the value
/// that the user has set. The key is used to store the setting in the global
/// settings [`Map`](super::Map) and needs to be unique across all types of
/// settings.
#[inline]
pub fn add_bool(key: &str, description: &str, default_value: bool) -> bool {
    // SAFETY: We provide valid pointers and lengths to key and description.
    // They are also guaranteed to be valid UTF-8 strings.
    unsafe {
        sys::user_settings_add_bool(
            key.as_ptr(),
            key.len(),
            description.as_ptr(),
            description.len(),
            default_value,
        )
    }
}

/// Adds a new title widget to the settings GUI. This is used to group settings
/// together. The heading level determines the size of the title. The top level
/// titles use a heading level of 0. The key needs to be unique across all types
/// of settings.
#[inline]
pub fn add_title(key: &str, description: &str, heading_level: u32) {
    // SAFETY: We provide valid pointers and lengths to key and description.
    // They are also guaranteed to be valid UTF-8 strings.
    unsafe {
        sys::user_settings_add_title(
            key.as_ptr(),
            key.len(),
            description.as_ptr(),
            description.len(),
            heading_level,
        )
    }
}

/// Adds a new choice setting widget that the user can modify. This allows the
/// user to choose between various options. The key is used to store the setting
/// in the settings [`Map`](super::Map) and needs to be unique across all types
/// of settings. The description is what's shown to the user. The key of the
/// default option to show needs to be specified.
#[inline]
pub fn add_choice(key: &str, description: &str, default_item_key: &str) {
    // SAFETY: We provide valid pointers and lengths to key, description and
    // default_item_key. They are also guaranteed to be valid UTF-8 strings.
    unsafe {
        sys::user_settings_add_choice(
            key.as_ptr(),
            key.len(),
            description.as_ptr(),
            description.len(),
            default_item_key.as_ptr(),
            default_item_key.len(),
        )
    }
}

/// Adds a new option to a choice setting widget. The key needs to match the key
/// of the choice setting widget that it's supposed to be added to. The option
/// key is used as the value to store when the user chooses this option. The
/// description is what's shown to the user. Returns [`true`] if the option is
/// at this point in time chosen by the user.
#[inline]
pub fn add_choice_option(key: &str, option_key: &str, option_description: &str) -> bool {
    // SAFETY: We provide valid pointers and lengths to key, option_key and
    // option_description. They are also guaranteed to be valid UTF-8 strings.
    unsafe {
        sys::user_settings_add_choice_option(
            key.as_ptr(),
            key.len(),
            option_key.as_ptr(),
            option_key.len(),
            option_description.as_ptr(),
            option_description.len(),
        )
    }
}

/// Adds a new file selection setting that the user can modify.
/// This allows the user to select a file path to be stored at the key.
/// The filter can include `*` wildcards, for example `"*.txt"`,
/// and multiple patterns separated by `;` semicolons, like `"*.txt;*.md"`.
pub fn add_file_selection(key: &str, description: &str, filter: &str) {
    unsafe {
        sys::user_settings_add_file_selection(
            key.as_ptr(),
            key.len(),
            description.as_ptr(),
            description.len(),
            filter.as_ptr(),
            filter.len(),
        )
    }
}

/// Adds a tooltip to a setting widget based on its key. A tooltip is useful for
/// explaining the purpose of a setting to the user.
#[inline]
pub fn set_tooltip(key: &str, tooltip: &str) {
    // SAFETY: We provide valid pointers and lengths to key and tooltip. They
    // are also guaranteed to be valid UTF-8 strings.
    unsafe {
        sys::user_settings_set_tooltip(key.as_ptr(), key.len(), tooltip.as_ptr(), tooltip.len())
    }
}

/// A trait that can be derived to describe an entire settings GUI through a
/// struct declaration. Check the derive macro [`Gui`](macro@Gui) for more
/// information.
pub trait Gui {
    /// Registers the settings by adding all the widgets to the settings GUI and
    /// initializing the settings with the values that the user has set or their
    /// default values if they haven't been modified yet.
    fn register() -> Self;

    /// Updates the settings with the values that the user has set from the
    /// currently set global settings map.
    fn update(&mut self) {
        self.update_from(&Map::load());
    }

    /// Updates the settings with the values that the user has set from the
    /// settings map provided.
    fn update_from(&mut self, settings_map: &Map);
}

/// A settings widget that can be used as a field when defining a settings [`Gui`].
pub trait Widget {
    /// The arguments that are needed to register the widget.
    type Args: Default;
    /// Registers the widget with the given key, description and default value.
    /// Returns the value that the user has set or the default value if the user
    /// did not set a value.
    fn register(key: &str, description: &str, args: Self::Args) -> Self;
    /// Updates the value of the setting based on the value that the user has
    /// set in the provided settings map.
    fn update_from(&mut self, settings_map: &Map, key: &str, args: Self::Args);
}

/// The arguments that are needed to register a boolean setting. This is an
/// internal type that you don't need to worry about.
#[doc(hidden)]
#[derive(Default)]
#[non_exhaustive]
pub struct BoolArgs {
    /// The default value of the setting, in case the user didn't set it yet.
    pub default: bool,
}

impl Widget for bool {
    type Args = BoolArgs;

    #[inline]
    fn register(key: &str, description: &str, args: Self::Args) -> Self {
        add_bool(key, description, args.default)
    }

    #[inline]
    fn update_from(&mut self, settings_map: &Map, key: &str, args: Self::Args) {
        *self = settings_map
            .get(key)
            .and_then(|value| value.get_bool())
            .unwrap_or(args.default);
    }
}

/// A title that can be used to group settings together.
pub struct Title;

/// The arguments that are needed to register a title. This is an internal type
/// that you don't need to worry about.
#[doc(hidden)]
#[derive(Default)]
#[non_exhaustive]
pub struct TitleArgs {
    /// The heading level of the title. The top level titles use a heading level
    /// of 0.
    pub heading_level: u32,
}

impl Widget for Title {
    type Args = TitleArgs;

    #[inline]
    fn register(key: &str, description: &str, args: Self::Args) -> Self {
        add_title(key, description, args.heading_level);
        Self
    }

    #[inline]
    fn update_from(&mut self, _settings_map: &Map, _key: &str, _args: Self::Args) {}
}

impl<T: Copy + Widget> Widget for Pair<T> {
    type Args = T::Args;

    fn register(key: &str, description: &str, args: Self::Args) -> Self {
        let value = T::register(key, description, args);
        Pair {
            old: value,
            current: value,
        }
    }

    fn update_from(&mut self, settings_map: &Map, key: &str, args: Self::Args) {
        self.old = self.current;
        self.current.update_from(settings_map, key, args);
    }
}

/// A file selection widget.
#[cfg(feature = "alloc")]
pub struct FileSelection {
    /// The file path, as accessible through the WASI file system,
    /// so a Windows path of `C:\foo\bar.exe` would be represented
    /// as `/mnt/c/foo/bar.exe`.
    pub path: alloc::string::String,
    /// Whether the path just changed on this update.
    pub new_data: bool,
}

/// The arguments that are needed to register a file selection widget.
/// This is an internal type that you don't need to worry about.
#[cfg(feature = "alloc")]
#[doc(hidden)]
#[derive(Default)]
#[non_exhaustive]
pub struct FileSelectionArgs {
    /// A filter on which files are selectable.
    /// Can include `*` wildcards, for example `"*.txt"`,
    /// and multiple patterns separated by `;` semicolons, like `"*.txt;*.md"`.
    pub filter: &'static str,
}

#[cfg(feature = "alloc")]
impl Widget for FileSelection {
    type Args = FileSelectionArgs;

    fn register(key: &str, description: &str, args: Self::Args) -> Self {
        add_file_selection(key, description, args.filter);
        FileSelection {
            path: alloc::string::ToString::to_string(""),
            new_data: false,
        }
    }

    fn update_from(&mut self, settings_map: &Map, key: &str, _args: Self::Args) {
        let new_path = settings_map
            .get(key)
            .and_then(|v| v.get_string())
            .unwrap_or_default();
        if self.path != new_path {
            self.path = new_path;
            self.new_data = true;
        } else {
            self.new_data = false;
        }
    }
}
