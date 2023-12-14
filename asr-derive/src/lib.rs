use heck::ToTitleCase;
use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
    spanned::Spanned, Data, DataEnum, DataStruct, DeriveInput, Expr, ExprLit, Ident, Lit, Meta,
};

// FIXME: https://github.com/rust-lang/rust/issues/117463
#[allow(rustdoc::redundant_explicit_links)]
/// Implements the `Gui` trait for a struct that allows you to register its
/// fields as settings widgets and returns the struct with the user's settings
/// applied.
///
/// The name of each field is used as the key for the setting for storing it in
/// the global settings map and looking up the current value.
///
/// The first paragraph in the doc comment of each field is used as the
/// description of the setting. The rest of the doc comment is used as the
/// tooltip. If there is no doc comment, the name of the field is used as the
/// description (in title case).
///
/// # Example
///
/// ```no_run
/// #[derive(Gui)]
/// struct Settings {
///     /// General Settings
///     _general_settings: Title,
///     /// Use Game Time
///     ///
///     /// This is the tooltip.
///     use_game_time: bool,
/// }
/// ```
///
/// The type can then be used like so:
///
/// ```no_run
/// let mut settings = Settings::register();
///
/// loop {
///    settings.update();
///    // Do something with the settings.
/// }
/// ```
///
/// # Attributes
///
/// The default value of the setting normally matches the
/// [`Default`](core::default::Default) trait. If you want to specify a
/// different default you can specify it like so:
///
/// ```no_run
/// # struct Settings {
/// #[default = true]
/// foo: bool,
/// # }
/// ```
///
/// The heading level of a title can be specified to form a hierarchy. The top
/// level titles use a heading level of 0. It is also the default heading level.
/// You can specify a different heading level like so:
///
/// ```no_run
/// # struct Settings {
/// #[heading_level = 2]
/// _title: Title,
/// # }
/// ```
///
/// A file selection filter can include `*` wildcards, for example `"*.txt"`,
/// and multiple patterns separated by `;` semicolons, like `"*.txt;*.md"`:
///
/// ```no_run
/// # struct Settings {
/// #[filter = "*.txt;*.md"]
/// text_file: FileSelection,
/// # }
/// ```
///
/// # Choices
///
/// You can derive `Gui` for an enum to create a choice widget. You can mark one
/// of the variants as the default by adding the `#[default]` attribute to it.
///
/// ```no_run
/// #[derive(Gui)]
/// enum Category {
///     /// Any%
///     AnyPercent,
///     /// Glitchless
///     Glitchless,
///     /// 100%
///     #[default]
///     HundredPercent,
/// }
/// ```
///
/// You can then use it as a widget like so:
///
/// ```no_run
/// #[derive(Gui)]
/// struct Settings {
///     /// Category
///     category: Category,
/// }
/// ```
///
/// # Tracking changes
///
/// You can track changes to a setting by wrapping the widget type in a `Pair`.
/// It acts like the widget by itself, but also keeps track of the previous
/// value when you call `update` on the struct.
///
/// ```no_run
/// use asr::watcher::Pair;
///
/// #[derive(Gui)]
/// struct Settings {
///     /// Use Game Time
///     use_game_time: Pair<bool>,
/// }
/// ```
#[proc_macro_derive(Gui, attributes(default, heading_level, filter))]
pub fn settings_macro(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();

    match ast.data {
        Data::Struct(s) => generate_struct_settings(ast.ident, s),
        Data::Enum(e) => generate_enum_settings(ast.ident, e),
        _ => panic!("Only structs and enums are supported"),
    }
}

fn generate_struct_settings(struct_name: Ident, struct_data: DataStruct) -> TokenStream {
    let mut field_names = Vec::new();
    let mut field_name_strings = Vec::new();
    let mut field_descs = Vec::new();
    let mut field_tooltips = Vec::new();
    let mut field_tys = Vec::new();
    let mut args_init = Vec::new();
    for field in struct_data.fields {
        let ident = field.ident.clone().unwrap();
        let ident_name = ident.to_string();
        field_names.push(ident);
        field_tys.push(field.ty);
        let mut doc_string = String::new();
        let mut tooltip_string = String::new();
        let mut is_in_tooltip = false;
        for attr in &field.attrs {
            let Meta::NameValue(nv) = &attr.meta else {
                continue;
            };
            let Some(ident) = nv.path.get_ident() else {
                continue;
            };
            if ident != "doc" {
                continue;
            }
            let Expr::Lit(ExprLit {
                lit: Lit::Str(s), ..
            }) = &nv.value
            else {
                continue;
            };
            let value = s.value();
            let value = value.trim();
            let target_string = if is_in_tooltip {
                &mut tooltip_string
            } else {
                &mut doc_string
            };
            if !target_string.is_empty() {
                if value.is_empty() {
                    if !is_in_tooltip {
                        is_in_tooltip = true;
                        continue;
                    }
                    target_string.push('\n');
                } else if !target_string.ends_with(|c: char| c.is_whitespace()) {
                    target_string.push(' ');
                }
            }
            target_string.push_str(&value);
        }
        if doc_string.is_empty() {
            doc_string = ident_name.to_title_case();
        }

        field_descs.push(doc_string);
        field_tooltips.push(if tooltip_string.is_empty() {
            quote! {}
        } else {
            quote! { asr::settings::gui::set_tooltip(#ident_name, #tooltip_string); }
        });
        field_name_strings.push(ident_name);

        let args = field
            .attrs
            .iter()
            .filter_map(|x| {
                let Meta::NameValue(nv) = &x.meta else {
                    return None;
                };
                let span = nv.span();
                if nv.path.is_ident("default") {
                    let value = &nv.value;
                    Some(quote_spanned! { span => args.default = #value; })
                } else if nv.path.is_ident("heading_level") {
                    let value = &nv.value;
                    Some(quote_spanned! { span => args.heading_level = #value; })
                } else if nv.path.is_ident("filter") {
                    let value = &nv.value;
                    Some(quote_spanned! { span => args.filter = #value; })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        args_init.push(quote! { #(#args)* });
    }

    quote! {
        impl asr::settings::Gui for #struct_name {
            fn register() -> Self {
                Self {
                    #(#field_names: {
                        let mut args = <#field_tys as asr::settings::gui::Widget>::Args::default();
                        #args_init
                        let mut value = asr::settings::gui::Widget::register(#field_name_strings, #field_descs, args);
                        #field_tooltips
                        value
                    },)*
                }
            }

            fn update_from(&mut self, settings_map: &asr::settings::Map) {
                #({
                    let mut args = <#field_tys as asr::settings::gui::Widget>::Args::default();
                    #args_init
                    asr::settings::gui::Widget::update_from(&mut self.#field_names, settings_map, #field_name_strings, args);
                })*
            }

            fn update(&mut self) {
                self.update_from(&asr::settings::Map::load());
            }
        }
    }
    .into()
}

fn generate_enum_settings(enum_name: Ident, enum_data: DataEnum) -> TokenStream {
    let mut variant_names = Vec::new();
    let mut variant_name_strings = Vec::new();
    let mut variant_descs = Vec::new();
    let mut default_index = None;
    for (index, variant) in enum_data.variants.into_iter().enumerate() {
        let ident = variant.ident.clone();
        let ident_name = ident.to_string();
        variant_names.push(ident);
        let mut doc_string = String::new();
        let mut tooltip_string = String::new();
        let mut is_in_tooltip = false;
        for attr in &variant.attrs {
            let Meta::NameValue(nv) = &attr.meta else {
                continue;
            };
            let Some(ident) = nv.path.get_ident() else {
                continue;
            };
            if ident != "doc" {
                continue;
            }
            let Expr::Lit(ExprLit {
                lit: Lit::Str(s), ..
            }) = &nv.value
            else {
                continue;
            };
            let value = s.value();
            let value = value.trim();
            let target_string = if is_in_tooltip {
                &mut tooltip_string
            } else {
                &mut doc_string
            };
            if !target_string.is_empty() {
                if value.is_empty() {
                    if !is_in_tooltip {
                        is_in_tooltip = true;
                        continue;
                    }
                    target_string.push('\n');
                } else if !target_string.ends_with(|c: char| c.is_whitespace()) {
                    target_string.push(' ');
                }
            }
            target_string.push_str(&value);
        }
        if doc_string.is_empty() {
            doc_string = ident_name.to_title_case();
        }

        variant_descs.push(doc_string);
        variant_name_strings.push(ident_name);

        let is_default = variant.attrs.iter().any(|x| {
            let Meta::Path(path) = &x.meta else {
                return false;
            };
            path.is_ident("default")
        });

        if is_default {
            if default_index.is_some() {
                panic!("Only one variant can be marked as default");
            }
            default_index = Some(index);
        }
    }

    let default_index = default_index.unwrap_or_default();

    let default_option = &variant_names[default_index];
    let default_option_key = &variant_name_strings[default_index];

    let longest_string = variant_name_strings
        .iter()
        .map(|x| x.len())
        .max()
        .unwrap_or_default();

    quote! {
        impl asr::settings::gui::Widget for #enum_name {
            type Args = ();

            #[inline]
            fn register(key: &str, description: &str, args: Self::Args) -> Self {
                asr::settings::gui::add_choice(key, description, #default_option_key);
                let mut v = Self::#default_option;
                #(if asr::settings::gui::add_choice_option(key, #variant_name_strings, #variant_descs) {
                    v = Self::#variant_names;
                })*
                v
            }

            #[inline]
            fn update_from(&mut self, settings_map: &asr::settings::Map, key: &str, args: Self::Args) {
                let Some(option_key) = settings_map.get(key).and_then(|v| v.get_array_string::<#longest_string>()?.ok()) else {
                    *self = Self::#default_option;
                    return;
                };
                *self = match &*option_key {
                    #(#variant_name_strings => Self::#variant_names,)*
                    _ => Self::#default_option,
                };
            }
        }
    }
    .into()
}

/// Generates an implementation of the `FromEndian` trait for a struct. This
/// allows converting values from a given endianness to the host's endianness.
///
/// # Example
///
/// ```no_run
/// #[derive(FromEndian)]
/// struct MyStruct {
///     a: u32,
///     b: u16,
/// }
/// ```
///
/// This will generate the following code:
///
/// ```no_run
/// impl FromEndian for MyStruct {
///     fn from_be(&self) -> Self {
///         Self {
///             a: self.a.from_be(),
///             b: self.b.from_be(),
///         }
///     }
///     fn from_le(&self) -> Self {
///         Self {
///             a: self.a.from_le(),
///             b: self.b.from_le(),
///         }
///     }
/// }
/// ```
#[proc_macro_derive(FromEndian)]
pub fn from_endian_macro(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();

    let struct_data = match ast.data {
        Data::Struct(s) => s,
        _ => panic!("Only structs are supported"),
    };

    let struct_name = ast.ident;

    let mut field_names = Vec::new();
    for field in struct_data.fields {
        field_names.push(field.ident);
    }

    quote! {
        impl asr::primitives::dynamic_endian::FromEndian for #struct_name {
            fn from_be(&self) -> Self {
                Self {
                    #(#field_names: asr::primitives::dynamic_endian::FromEndian::from_be(
                        &self.#field_names,
                    ),)*
                }
            }
            fn from_le(&self) -> Self {
                Self {
                    #(#field_names: asr::primitives::dynamic_endian::FromEndian::from_le(
                        &self.#field_names,
                    ),)*
                }
            }
        }
    }
    .into()
}

#[cfg(feature = "unity")]
mod unity;

/// A derive macro that can be used to bind to a .NET class. This allows reading
/// the contents of an instance of the class described by the struct from a
/// process. Each field must match the name of the field in the class exactly
/// (or alternatively renamed with the `#[rename = "..."]` attribute) and needs
/// to be of a type that can be read from a process. Fields can be marked as
/// static with the `#[static_field]` attribute.
///
/// # Example
///
/// ```no_run
/// #[derive(Class)]
/// struct Timer {
///     #[rename = "currentLevelTime"]
///     level_time: f32,
///     #[static_field]
///     foo: bool,
/// }
/// ```
///
/// This will bind to a .NET class of the following shape:
///
/// ```csharp
/// class Timer
/// {
///     float currentLevelTime;
///     static bool foo;
///     // ...
/// }
/// ```
///
/// The class can then be bound to the process like so:
///
/// ```no_run
/// let timer_class = Timer::bind(&process, &module, &image).await;
/// ```
///
/// Once you have an instance, you can read the instance from the process like
/// so:
///
/// ```no_run
/// if let Ok(timer) = timer_class.read(&process, timer_instance) {
///     // Do something with the instance.
/// }
/// ```
///
/// If only static fields are present, the `read` method does not take an
/// instance argument.
#[cfg(feature = "unity")]
#[proc_macro_derive(Il2cppClass, attributes(static_field, rename))]
pub fn il2cpp_class_binding(input: TokenStream) -> TokenStream {
    unity::process(input, quote! { asr::game_engine::unity::il2cpp })
}

/// A derive macro that can be used to bind to a .NET class. This allows reading
/// the contents of an instance of the class described by the struct from a
/// process. Each field must match the name of the field in the class exactly
/// (or alternatively renamed with the `#[rename = "..."]` attribute) and needs
/// to be of a type that can be read from a process. Fields can be marked as
/// static with the `#[static_field]` attribute.
///
/// # Example
///
/// ```no_run
/// #[derive(Class)]
/// struct Timer {
///     #[rename = "currentLevelTime"]
///     level_time: f32,
///     #[static_field]
///     foo: bool,
/// }
/// ```
///
/// This will bind to a .NET class of the following shape:
///
/// ```csharp
/// class Timer
/// {
///     float currentLevelTime;
///     static bool foo;
///     // ...
/// }
/// ```
///
/// The class can then be bound to the process like so:
///
/// ```no_run
/// let timer_class = Timer::bind(&process, &module, &image).await;
/// ```
///
/// Once you have an instance, you can read the instance from the process like
/// so:
///
/// ```no_run
/// if let Ok(timer) = timer_class.read(&process, timer_instance) {
///     // Do something with the instance.
/// }
/// ```
///
/// If only static fields are present, the `read` method does not take an
/// instance argument.
#[cfg(feature = "unity")]
#[proc_macro_derive(MonoClass, attributes(static_field, rename))]
pub fn mono_class_binding(input: TokenStream) -> TokenStream {
    unity::process(input, quote! { asr::game_engine::unity::mono })
}
