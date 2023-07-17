use heck::ToTitleCase;
use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, Data, DeriveInput, Expr, ExprLit, Lit, Meta};

/// Generates a `register` method for a struct that automatically registers its
/// fields as settings and returns the struct with the user's settings applied.
///
/// # Example
///
/// ```no_run
/// #[derive(Settings)]
/// struct MySettings {
///     /// General Settings
///     _general_settings: Title,
///     /// Use Game Time
///     ///
///     /// This is the tooltip.
///     use_game_time: bool,
/// }
/// ```
///
/// This will generate the following code:
///
/// ```no_run
/// impl MySettings {
///    pub fn register() -> Self {
///       asr::user_settings::add_title("_general_settings", "General Settings", 0);
///       let use_game_time = asr::user_settings::add_bool("use_game_time", "Use Game Time", false);
///       asr::user_settings::set_tooltip("use_game_time", "This is the tooltip.");
///       Self { use_game_time }
///    }
/// }
/// ```
#[proc_macro_derive(Settings, attributes(default, heading_level))]
pub fn settings_macro(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();

    let struct_data = match ast.data {
        Data::Struct(s) => s,
        _ => panic!("Only structs are supported"),
    };

    let struct_name = ast.ident;

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
            let Meta::NameValue(nv) = &attr.meta else { continue };
            let Some(ident) =  nv.path.get_ident() else { continue };
            if ident != "doc" {
                continue;
            }
            let Expr::Lit(ExprLit {
                lit: Lit::Str(s), ..
            }) = &nv.value else { continue };
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
            quote! { asr::user_settings::set_tooltip(#ident_name, #tooltip_string); }
        });
        field_name_strings.push(ident_name);

        let args = field
            .attrs
            .iter()
            .filter_map(|x| {
                let Meta::NameValue(nv) = &x.meta else { return None };
                let span = nv.span();
                if nv.path.is_ident("default") {
                    let value = &nv.value;
                    Some(quote_spanned! { span => args.default = #value; })
                } else if nv.path.is_ident("heading_level") {
                    let value = &nv.value;
                    Some(quote_spanned! { span => args.heading_level = #value; })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        args_init.push(quote! { #(#args)* });
    }

    quote! {
        impl #struct_name {
            pub fn register() -> Self {
                Self {
                    #(#field_names: {
                        let mut args = <#field_tys as asr::user_settings::Setting>::Args::default();
                        #args_init
                        let mut value = asr::user_settings::Setting::register(#field_name_strings, #field_descs, args);
                        #field_tooltips
                        value
                    },)*
                }
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
/// and needs to be of a type that can be read from a process.
///
/// # Example
///
/// ```no_run
/// #[derive(Class)]
/// struct Timer {
///     currentLevelTime: f32,
///     timerStopped: bool,
/// }
/// ```
///
/// This will bind to a .NET class of the following shape:
///
/// ```csharp
/// class Timer
/// {
///     float currentLevelTime;
///     bool timerStopped;
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
#[cfg(feature = "unity")]
#[proc_macro_derive(Il2cppClass)]
pub fn il2cpp_class_binding(input: TokenStream) -> TokenStream {
    unity::process(input, quote! { asr::game_engine::unity::il2cpp })
}

/// A derive macro that can be used to bind to a .NET class. This allows reading
/// the contents of an instance of the class described by the struct from a
/// process. Each field must match the name of the field in the class exactly
/// and needs to be of a type that can be read from a process.
///
/// # Example
///
/// ```no_run
/// #[derive(Class)]
/// struct Timer {
///     currentLevelTime: f32,
///     timerStopped: bool,
/// }
/// ```
///
/// This will bind to a .NET class of the following shape:
///
/// ```csharp
/// class Timer
/// {
///     float currentLevelTime;
///     bool timerStopped;
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
#[cfg(feature = "unity")]
#[proc_macro_derive(MonoClass)]
pub fn mono_class_binding(input: TokenStream) -> TokenStream {
    unity::process(input, quote! { asr::game_engine::unity::mono })
}
