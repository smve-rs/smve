use proc_macro::{TokenStream};
use quote::quote;
use syn::{DeriveInput, parse_macro_input, parse_quote};

pub fn derive_extract_component(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    
    // Require `Clone` for the component
    ast.generics.make_where_clause().predicates.push(parse_quote! { Self: Clone });
    
    let struct_name = &ast.ident;
    
    let filter = if let Some(attribute) = ast
        .attrs
        .iter()
        .find(|a| a.path().is_ident("extract_component_filter"))
    {
        // The argument to the attribute should be a type, otherwise compile error
        let filter = match attribute.parse_args::<syn::Type>() {
            Ok(filter) => filter,
            Err(e) => return e.to_compile_error().into(),
        };
        
        quote! {
            #filter
        }
    } else {
        quote! {
            ()
        }
    };

    TokenStream::from(quote! {
        impl crate::core::graphics::extract::utils::extract_component::ExtractComponent for #struct_name {
            type QueryData = &'static Self;
            type QueryFilter = #filter;
            type Out = Self;

            fn extract_component(item: bevy_ecs::query::QueryItem<'_, Self::QueryData>) -> Option<Self::Out> {
                Some(item.clone())
            }
        }
    })
}