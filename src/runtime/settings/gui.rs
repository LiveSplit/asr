//! This module allows you to add settings widgets to the settings GUI that the
//! user can modify.

use core::mem;

#[cfg(feature = "derive")]
pub use asr_derive::Gui;

#[cfg(feature = "alloc")]
use alloc::borrow::ToOwned;

use crate::{runtime::sys, watcher::Pair};

#[cfg(feature = "alloc")]
use crate::sync::RacyCell;

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

#[cfg(feature = "alloc")]
type ButtonHandler = (alloc::string::String, fn());

#[cfg(feature = "alloc")]
static BUTTON_HANDLERS: RacyCell<alloc::vec::Vec<ButtonHandler>> =
    RacyCell::new(alloc::vec::Vec::new());

#[cfg(feature = "alloc")]
fn register_button_handler(key: &str, on_click: fn()) {
    // SAFETY: The auto splitter runtime is single-threaded, so there are no
    // other references to the handler list while we mutate it.
    let handlers = unsafe { &mut *BUTTON_HANDLERS.get_mut() };
    if let Some((_, slot)) = handlers.iter_mut().find(|(k, _)| k == key) {
        *slot = on_click;
        return;
    }
    handlers.push((key.into(), on_click));
}

/// Adds a new button to the settings GUI. The key needs to be unique across
/// all types of settings and is not persisted in the settings
/// [`Map`](super::Map). The description is what's shown to the user. When the
/// user clicks the button, `on_click` is called.
///
/// Click handlers are synchronous. They may load and store the settings
/// [`Map`](super::Map), but they cannot `.await`.
///
/// Clicks are only delivered if you export the host entry point with
/// [`export_settings_buttons`](macro@crate::export_settings_buttons). Without that macro, the host ignores clicks.
///
/// This requires the `alloc` feature.
///
/// # Example
///
/// ```ignore
/// fn clear_counters() {
///     let mut map = asr::settings::Map::load();
///     map.insert("deaths", 0i64);
///     map.store();
/// }
///
/// asr::settings::gui::add_button("clear_counters", "Clear Counters", clear_counters);
///
/// asr::export_settings_buttons!();
/// ```
#[cfg(feature = "alloc")]
#[inline]
pub fn add_button(key: &str, description: &str, on_click: fn()) {
    // SAFETY: We provide valid pointers and lengths to key and description.
    // They are also guaranteed to be valid UTF-8 strings.
    unsafe {
        sys::user_settings_add_button(
            key.as_ptr(),
            key.len(),
            description.as_ptr(),
            description.len(),
        )
    }
    register_button_handler(key, on_click);
}

/// Dispatches a settings button click to the handler registered for `key`.
/// This is called by [`export_settings_buttons`](macro@crate::export_settings_buttons).
#[cfg(feature = "alloc")]
pub fn dispatch_button(key: &str) {
    // SAFETY: The auto splitter runtime is single-threaded. The function
    // pointer is copied out so the handler can run without aliasing the
    // handler list, including if it registers further buttons.
    let on_click = unsafe {
        (*BUTTON_HANDLERS.get())
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, f)| *f)
    };
    if let Some(on_click) = on_click {
        on_click();
    }
}

/// Reads the key of the settings button that is currently being invoked.
#[cfg(feature = "alloc")]
#[doc(hidden)]
pub fn get_button_key() -> Option<alloc::string::String> {
    // SAFETY: Calling with a null pointer and 0 length returns the required
    // length. We then allocate a buffer and call again. After a successful
    // call, the buffer contains `len` bytes of valid UTF-8.
    unsafe {
        let mut len = 0;
        let success = sys::user_settings_get_button_key(core::ptr::null_mut(), &mut len);
        if len == 0 && !success {
            return None;
        }
        let mut buf = alloc::vec::Vec::with_capacity(len);
        let success = sys::user_settings_get_button_key(buf.as_mut_ptr(), &mut len);
        if !success {
            return None;
        }
        buf.set_len(len);
        Some(alloc::string::String::from_utf8_unchecked(buf))
    }
}

/// Exports the `on_settings_button` entry point so the runtime can deliver
/// settings button clicks to handlers registered with [`add_button`] or
/// `#[button(on_click = ...)]`.
///
/// The generated export takes no arguments. It pulls the clicked button's key
/// through `user_settings_get_button_key` and then calls [`dispatch_button`].
/// Authors never see `key_ptr` / `key_len` on the export. This is the
/// import-pull ABI used by `livesplit-auto-splitting`.
///
/// Without this macro, the host ignores button clicks. Requires the `alloc`
/// feature.
///
/// # Example
///
/// ```ignore
/// asr::export_settings_buttons!();
///
/// fn clear_counters() {
///     let mut map = asr::settings::Map::load();
///     map.insert("deaths", 0i64);
///     map.store();
/// }
///
/// #[no_mangle]
/// pub extern "C" fn update() {
///     asr::settings::gui::add_button("clear_counters", "Clear Counters", clear_counters);
/// }
/// ```
#[macro_export]
macro_rules! export_settings_buttons {
    () => {
        /// Called by the runtime when the user clicks a settings button.
        ///
        /// # Safety
        /// This is invoked by the auto splitting runtime.
        #[no_mangle]
        pub unsafe extern "C" fn on_settings_button() {
            if let Some(key) = $crate::settings::gui::get_button_key() {
                $crate::settings::gui::dispatch_button(&key);
            }
        }
    };
}

/// Adds a new choice setting widget that the user can modify. This allows the
/// user to choose between various options. The key is used to store the setting
/// in the settings [`Map`](super::Map) and needs to be unique across all types
/// of settings. The description is what's shown to the user. The key of the
/// default option to show needs to be specified.
#[inline]
pub fn add_choice(key: &str, description: &str, default_option_key: &str) {
    // SAFETY: We provide valid pointers and lengths to key, description and
    // default_option_key. They are also guaranteed to be valid UTF-8 strings.
    unsafe {
        sys::user_settings_add_choice(
            key.as_ptr(),
            key.len(),
            description.as_ptr(),
            description.len(),
            default_option_key.as_ptr(),
            default_option_key.len(),
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

/// Adds a new file select setting that the user can modify. This allows the
/// user to choose a file from the file system. The key is used to store the
/// path of the file in the settings map and needs to be unique across all types
/// of settings. The description is what's shown to the user. The path is a path
/// that is accessible through the WASI file system, so a Windows path of
/// `C:\foo\bar.exe` would be stored as `/mnt/c/foo/bar.exe`.
#[inline]
pub fn add_file_select(key: &str, description: &str) {
    // SAFETY: We provide valid pointers and lengths to key and description.
    // They are also guaranteed to be valid UTF-8 strings.
    unsafe {
        sys::user_settings_add_file_select(
            key.as_ptr(),
            key.len(),
            description.as_ptr(),
            description.len(),
        )
    }
}

/// Adds a filter to a file select setting. The key needs to match the key of
/// the file select setting that it's supposed to be added to. The description
/// is what's shown to the user for the specific filter. The pattern is a [glob
/// pattern](https://en.wikipedia.org/wiki/Glob_(programming)) that is used to
/// filter the files. The pattern generally only supports `*` wildcards, not `?`
/// or brackets. This may however differ between frontends. Additionally `;`
/// can't be used in Windows's native file dialog if it's part of the pattern.
/// Multiple patterns may be specified by separating them with ASCII space
/// characters. There are operating systems where glob patterns are not
/// supported. A best effort lookup of the fitting MIME type may be used by a
/// frontend on those operating systems instead.
#[inline]
pub fn add_file_select_name_filter(key: &str, description: Option<&str>, pattern: &str) {
    // SAFETY: We provide valid pointers and lengths to key, description and
    // pattern. They are also guaranteed to be valid UTF-8 strings. The
    // description is provided as a null pointer in case it is `None` to
    // indicate that no description is provided.
    unsafe {
        let (desc_ptr, desc_len) = match description {
            Some(desc) => (desc.as_ptr(), desc.len()),
            None => (core::ptr::null(), 0),
        };
        sys::user_settings_add_file_select_name_filter(
            key.as_ptr(),
            key.len(),
            desc_ptr,
            desc_len,
            pattern.as_ptr(),
            pattern.len(),
        )
    }
}

/// Adds a filter to a file select setting. The key needs to match the key
/// of the file select setting that it's supposed to be added to. The MIME
/// type is what's used to filter the files. Most operating systems do not
/// support MIME types, but the frontends are encouraged to look up the file
/// extensions that are associated with the MIME type and use those as a
/// filter in those cases. You may also use wildcards as part of the MIME
/// types such as `image/*`. The support likely also varies between
/// frontends however.
#[inline]
pub fn add_file_select_mime_filter(key: &str, mime_type: &str) {
    // SAFETY: We provide valid pointers and lengths to key and mime_type.
    // They are also guaranteed to be valid UTF-8 strings.
    unsafe {
        sys::user_settings_add_file_select_mime_filter(
            key.as_ptr(),
            key.len(),
            mime_type.as_ptr(),
            mime_type.len(),
        )
    }
}

/// Adds a new text input setting widget that the user can modify. The key is
/// used to store the text in the settings [`Map`](super::Map) and needs to be
/// unique across all types of settings. The description is what's shown to the
/// user. The default value is used when the user hasn't set a value yet.
#[inline]
pub fn add_text_input(key: &str, description: &str, default_value: &str) {
    // SAFETY: We provide valid pointers and lengths to key, description and
    // default_value. They are also guaranteed to be valid UTF-8 strings.
    unsafe {
        sys::user_settings_add_text_input(
            key.as_ptr(),
            key.len(),
            description.as_ptr(),
            description.len(),
            default_value.as_ptr(),
            default_value.len(),
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

/// A button that the user can click. Buttons are not persisted in the settings
/// [`Map`](super::Map).
///
/// Click handlers are synchronous. They may load and store the settings
/// [`Map`](super::Map), but they cannot `.await`.
///
/// The field requires `#[button(on_click = ...)]`. Clicks are only delivered if
/// you also call [`export_settings_buttons`](macro@crate::export_settings_buttons). Without that macro, the host
/// ignores clicks.
///
/// This requires the `alloc` feature.
///
/// # Example
///
/// ```ignore
/// fn clear_counters() {
///     let mut map = asr::settings::Map::load();
///     map.insert("deaths", 0i64);
///     map.store();
/// }
///
/// #[derive(Gui)]
/// struct Settings {
///     /// Clear Counters
///     #[button(on_click = clear_counters)]
///     clear_counters: Button,
/// }
///
/// asr::export_settings_buttons!();
/// ```
#[cfg(feature = "alloc")]
pub struct Button;

/// The arguments that are needed to register a button. This is an internal type
/// that you don't need to worry about.
#[cfg(feature = "alloc")]
#[doc(hidden)]
#[non_exhaustive]
pub struct ButtonArgs {
    /// The function to call when the user clicks the button.
    pub on_click: fn(),
}

#[cfg(feature = "alloc")]
impl Default for ButtonArgs {
    #[inline]
    fn default() -> Self {
        Self { on_click: || {} }
    }
}

#[cfg(feature = "alloc")]
impl Widget for Button {
    type Args = ButtonArgs;

    #[inline]
    fn register(key: &str, description: &str, args: Self::Args) -> Self {
        add_button(key, description, args.on_click);
        Self
    }

    #[inline]
    fn update_from(&mut self, _settings_map: &Map, _key: &str, _args: Self::Args) {}
}

impl<T: Clone + Widget> Widget for Pair<T> {
    type Args = T::Args;

    fn register(key: &str, description: &str, args: Self::Args) -> Self {
        let value = T::register(key, description, args);
        Pair {
            old: value.clone(),
            current: value,
        }
    }

    fn update_from(&mut self, settings_map: &Map, key: &str, args: Self::Args) {
        mem::swap(&mut self.old, &mut self.current);
        self.current.update_from(settings_map, key, args);
    }
}

/// A file select widget.
///
/// # Example
///
/// ```ignore
/// # struct Settings {
/// #[filter(
///     // File name patterns with names
///     ("PNG images", "*.png"),
///     // Multiple patterns separated by space
///     ("Rust files", "*.rs Cargo.*"),
///     // The name is optional
///     (_, "*.md"),
///     // MIME types
///     "text/plain",
///     // MIME types with wildcards
///     "image/*",
/// )]
/// text_file: FileSelect,
/// # }
/// ```
#[derive(Clone, PartialEq, Eq)]
#[cfg(feature = "alloc")]
pub struct FileSelect {
    /// The file path, as accessible through the WASI file system,
    /// so a Windows path of `C:\foo\bar.exe` would be represented
    /// as `/mnt/c/foo/bar.exe`.
    pub path: alloc::string::String,
}

/// The arguments that are needed to register a file selection widget.
/// This is an internal type that you don't need to worry about.
#[cfg(feature = "alloc")]
#[doc(hidden)]
#[derive(Default)]
#[non_exhaustive]
pub struct FileSelectArgs {
    pub filter: &'static [FileSelectFilter],
}

#[cfg(feature = "alloc")]
#[doc(hidden)]
pub enum FileSelectFilter {
    NamePattern(Option<&'static str>, &'static str),
    MimeType(&'static str),
}

#[cfg(feature = "alloc")]
impl Widget for FileSelect {
    type Args = FileSelectArgs;

    fn register(key: &str, description: &str, args: Self::Args) -> Self {
        add_file_select(key, description);
        for filter in args.filter {
            match filter {
                FileSelectFilter::NamePattern(desc, pattern) => {
                    add_file_select_name_filter(key, *desc, pattern)
                }
                FileSelectFilter::MimeType(mime) => add_file_select_mime_filter(key, mime),
            }
        }
        let mut this = FileSelect {
            path: alloc::string::String::new(),
        };
        this.update_from(&Map::load(), key, args);
        this
    }

    fn update_from(&mut self, settings_map: &Map, key: &str, _args: Self::Args) {
        if let Some(value) = settings_map.get(key) {
            value.get_string_into(&mut self.path);
        } else {
            self.path.clear();
        }
    }
}

/// A text input widget.
///
/// This is useful for settings where the user needs to enter arbitrary text,
/// such as a scene or level name to split on.
///
/// # Example
///
/// ```ignore
/// #[derive(Gui)]
/// struct Settings {
///     /// Screen Name
///     ///
///     /// Split when entering a scene with this name.
///     #[default = "prologue"]
///     level_name: TextInput,
/// }
///
/// // In your auto splitter:
/// if current_scene == settings.level_name.value {
///     timer::split();
/// }
/// ```
#[derive(Clone, PartialEq, Eq)]
#[cfg(feature = "alloc")]
pub struct TextInput {
    /// The text entered by the user, or the default if they haven't changed it.
    pub value: alloc::string::String,
}

/// The arguments that are needed to register a text input widget. This is an
/// internal type that you don't need to worry about.
#[cfg(feature = "alloc")]
#[doc(hidden)]
#[derive(Default)]
#[non_exhaustive]
pub struct TextInputArgs {
    /// The default value of the setting, in case the user didn't set it yet.
    pub default: &'static str,
}

#[cfg(feature = "alloc")]
impl Widget for TextInput {
    type Args = TextInputArgs;

    fn register(key: &str, description: &str, args: Self::Args) -> Self {
        add_text_input(key, description, args.default);
        let mut this = TextInput {
            value: alloc::string::String::new(),
        };
        this.update_from(&Map::load(), key, args);
        this
    }

    fn update_from(&mut self, settings_map: &Map, key: &str, args: Self::Args) {
        if let Some(value) = settings_map.get(key) {
            value.get_string_into(&mut self.value);
        } else {
            args.default.clone_into(&mut self.value);
        }
    }
}
