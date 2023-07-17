use proc_macro::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{Data, DeriveInput, Ident};

pub fn process(input: TokenStream, mono_module: impl ToTokens) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();

    let struct_data = match ast.data {
        Data::Struct(s) => s,
        _ => panic!("Only structs are supported"),
    };

    let struct_name = ast.ident;
    let stuct_name_string = struct_name.to_string();

    let binding_name = Ident::new(&format!("{struct_name}Binding"), struct_name.span());

    let mut field_names = Vec::new();
    let mut field_name_strings = Vec::new();
    let mut field_types = Vec::new();
    let mut field_reads = Vec::new();
    for field in struct_data.fields {
        let field_name = field.ident.clone().unwrap();
        let span = field_name.span();
        field_reads.push(quote_spanned! { span =>
            process.read(instance + self.#field_name).map_err(drop)?
        });
        field_names.push(field_name);
        field_name_strings.push(field.ident.clone().unwrap().to_string());
        field_types.push(field.ty);
    }

    quote! {
        struct #binding_name {
            class: #mono_module::Class,
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
                    let #field_names = class.wait_get_field(process, module, #field_name_strings).await;
                )*

                #binding_name {
                    class,
                    #(#field_names,)*
                }
            }
        }

        impl #binding_name {
            fn class(&self) -> &#mono_module::Class {
                &self.class
            }

            fn read(&self, process: &asr::Process, instance: asr::Address) -> Result<#struct_name, ()> {
                Ok(#struct_name {#(
                    #field_names: #field_reads,
                )*})
            }
        }
    }
    .into()
}
