use heck::ToTitleCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Expr, ExprLit, Lit, LitStr, Meta};

#[proc_macro_derive(Settings, attributes(default))]
pub fn mono_class_binding(input: TokenStream) -> TokenStream {
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
                    let nv = match x.parse_meta().ok()? {
                        Meta::NameValue(nv) => nv,
                        _ => return None,
                    };
                    if nv.path.get_ident()? != "doc" {
                        return None;
                    }
                    let lit = match nv.lit {
                        Lit::Str(s) => LitStr::new(s.value().trim(), s.span()),
                        _ => return None,
                    };
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
                    let nv = match x.parse_meta().ok()? {
                        Meta::NameValue(nv) => nv,
                        _ => return None,
                    };
                    if nv.path.get_ident()? != "default" {
                        return None;
                    }
                    Some(Expr::Lit(ExprLit {
                        attrs: Vec::new(),
                        lit: nv.lit,
                    }))
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
