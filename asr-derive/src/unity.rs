use proc_macro::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{Data, DeriveInput, Expr, ExprLit, Ident, Lit, Meta};

pub fn process(input: TokenStream, mono_module: impl ToTokens) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();

    let struct_data = match ast.data {
        Data::Struct(s) => s,
        _ => panic!("Only structs are supported"),
    };

    let struct_name = ast.ident;
    let stuct_name_string = struct_name.to_string();

    let binding_name = Ident::new(&format!("{struct_name}Binding"), struct_name.span());

    let mut has_static = false;
    let mut is_fully_static = true;

    let mut field_names = Vec::new();
    let mut lookup_names = Vec::new();
    let mut field_types = Vec::new();
    let mut field_reads = Vec::new();
    for field in struct_data.fields {
        let field_name = field.ident.clone().unwrap();
        let span = field_name.span();
        let is_static = field.attrs.iter().any(|x| {
            let Meta::Path(path) = &x.meta else {
                return false;
            };
            path.is_ident("static_field")
        });
        field_reads.push(if is_static {
            quote_spanned! { span =>
                process.read(self.static_table + self.#field_name).map_err(drop)?
            }
        } else {
            quote_spanned! { span =>
                process.read(instance + self.#field_name).map_err(drop)?
            }
        });
        let lookup_name = field
            .attrs
            .iter()
            .find_map(|x| {
                let Meta::NameValue(name_value) = &x.meta else {
                    return None;
                };
                if !name_value.path.is_ident("rename") {
                    return None;
                }
                let Expr::Lit(ExprLit {
                    lit: Lit::Str(name),
                    ..
                }) = &name_value.value
                else {
                    return None;
                };
                Some(name.value())
            })
            .unwrap_or_else(|| field.ident.clone().unwrap().to_string());
        has_static |= is_static;
        is_fully_static &= is_static;
        field_names.push(field_name);
        lookup_names.push(lookup_name);
        field_types.push(field.ty);
    }

    let static_table_field = if has_static {
        quote! {
            static_table: asr::Address,
        }
    } else {
        quote! {}
    };

    let static_table_init = if has_static {
        quote! {
            static_table: class.wait_get_static_table(process, module).await,
        }
    } else {
        quote! {}
    };

    let maybe_instance_param = if is_fully_static {
        quote! {}
    } else {
        quote! { , instance: asr::Address }
    };

    quote! {
        struct #binding_name {
            class: #mono_module::Class,
            #static_table_field
            #(#field_names: u32,)*
        }

        impl #struct_name {
            async fn bind(
                process: &asr::Process,
                module: &#mono_module::Module,
                image: &#mono_module::Image,
            ) -> #binding_name {
                let class = image.wait_get_class(process, module, #stuct_name_string).await;

                #(
                    let #field_names = class.wait_get_field_offset(process, module, #lookup_names).await;
                )*

                #binding_name {
                    #static_table_init
                    class,
                    #(#field_names,)*
                }
            }
        }

        impl #binding_name {
            fn class(&self) -> &#mono_module::Class {
                &self.class
            }

            fn read(&self, process: &asr::Process #maybe_instance_param) -> Result<#struct_name, ()> {
                Ok(#struct_name {#(
                    #field_names: #field_reads,
                )*})
            }
        }
    }
    .into()
}
