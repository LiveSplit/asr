//! This module allows you to add settings that the user can modify.

use super::sys;

#[cfg(feature = "derive")]
pub use asr_derive::Settings;

/// Adds a new boolean setting that the user can modify. This will return either
/// the specified default value or the value that the user has set. The key is
/// used to store the setting and needs to be unique across all types of
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

/// Adds a new title to the user settings. This is used to group settings
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

/// Adds a tooltip to a setting based on its key. A tooltip is useful for
/// explaining the purpose of a setting to the user.
#[inline]
pub fn set_tooltip(key: &str, tooltip: &str) {
    // SAFETY: We provide valid pointers and lengths to key and description.
    // They are also guaranteed to be valid UTF-8 strings.
    unsafe {
        sys::user_settings_set_tooltip(key.as_ptr(), key.len(), tooltip.as_ptr(), tooltip.len())
    }
}

/// A type that can be registered as a user setting. This is an internal trait
/// that you don't need to worry about.
pub trait Setting {
    /// The arguments that are needed to register the setting.
    type Args: Default;
    /// Registers the setting with the given key, description and default value.
    /// Returns the value that the user has set or the default value if the user
    /// did not set a value.
    fn register(key: &str, description: &str, args: Self::Args) -> Self;
}

/// The arguments that are needed to register a boolean setting. This is an
/// internal type that you don't need to worry about.
#[derive(Default)]
#[non_exhaustive]
pub struct BoolArgs {
    /// The default value of the setting, in case the user didn't set it yet.
    pub default: bool,
}

impl Setting for bool {
    type Args = BoolArgs;

    #[inline]
    fn register(key: &str, description: &str, args: Self::Args) -> Self {
        add_bool(key, description, args.default)
    }
}

/// A title that can be used to group settings together.
pub struct Title;

/// The arguments that are needed to register a title. This is an internal type
/// that you don't need to worry about.
#[derive(Default)]
#[non_exhaustive]
pub struct TitleArgs {
    /// The heading level of the title. The top level titles use a heading level
    /// of 0.
    pub heading_level: u32,
}

impl Setting for Title {
    type Args = TitleArgs;

    #[inline]
    fn register(key: &str, description: &str, args: Self::Args) -> Self {
        add_title(key, description, args.heading_level);
        Self
    }
}
