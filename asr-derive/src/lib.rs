use heck::ToTitleCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Expr, ExprLit, Lit, LitStr, Meta};

/// Generates a `register` method for a struct that automatically registers its
/// fields as settings and returns the struct with the user's settings applied.
///
/// # Example
///
/// ```no_run
/// #[derive(Settings)]
/// struct MySettings {
///     /// Use Game Time
///     use_game_time: bool,
/// }
/// ```
///
/// This will generate the following code:
///
/// ```no_run
/// impl MySettings {
///    fn register() -> Self {
///       let use_game_time = asr::Setting::register("use_game_time", "Use Game Time", false);
///       Self { use_game_time }
///    }
/// }
/// ```
#[proc_macro_derive(Settings, attributes(default))]
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
    let mut field_defaults = Vec::new();
    for field in struct_data.fields {
        let ident = field.ident.clone().unwrap();
        let ident_name = ident.to_string();
        let ident_span = ident.span();
        field_names.push(ident);
        field_descs.push(
            field
                .attrs
                .iter()
                .find_map(|x| {
                    let Meta::NameValue(nv) = &x.meta else { return None };
                    if nv.path.get_ident()? != "doc" {
                        return None;
                    }
                    let Expr::Lit(ExprLit {
                        lit: Lit::Str(s), ..
                    }) = &nv.value else { return None };
                    let lit = LitStr::new(s.value().trim(), s.span());
                    Some(Expr::Lit(ExprLit {
                        attrs: Vec::new(),
                        lit: Lit::Str(lit),
                    }))
                })
                .unwrap_or_else(|| {
                    Expr::Lit(ExprLit {
                        attrs: Vec::new(),
                        lit: Lit::Str(LitStr::new(&ident_name.to_title_case(), ident_span)),
                    })
                }),
        );
        field_name_strings.push(ident_name);
        field_defaults.push(
            field
                .attrs
                .iter()
                .find_map(|x| {
                    let Meta::NameValue(nv) = &x.meta else { return None };
                    if !nv.path.is_ident("default") {
                        return None;
                    }
                    Some(nv.value.clone())
                })
                .unwrap_or_else(|| {
                    syn::parse(quote! { ::core::default::Default::default() }.into()).unwrap()
                }),
        );
    }

    quote! {
        impl #struct_name {
            pub fn register() -> Self {
                Self {
                    #(#field_names: asr::Setting::register(#field_name_strings, #field_descs, #field_defaults),)*
                }
            }
        }
    }
    .into()
}
