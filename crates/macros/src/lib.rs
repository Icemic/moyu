mod bindgen;

use proc_macro::TokenStream;

#[cfg(feature = "js_runtime")]
#[proc_macro_attribute]
pub fn moyu_bindgen(args: TokenStream, func_body: TokenStream) -> TokenStream {
    bindgen::entry(args, func_body)
}

use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

/// It will automatically implement the `NodeBaseTrait` trait for the struct,
/// the `node_base` field name will be marked by attr `#[base]`.
/// Input:
/// ```ignore
/// #[derive(Node)]
/// pub struct Container {
///    #[base] node_base: NodeBase;
/// }
/// ```
/// Output:
/// ```ignore
/// impl NodeBaseTrait for Container {
///    fn as_any(&self) -> &dyn Any {
///       self
///   }
///   fn base(&self) -> &NodeBase {
///     &self.node_base
///   }
///
///   ...
/// }
/// ```
#[proc_macro_derive(Node, attributes(base))]
pub fn derive_node_attr(item: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);
    let name = &ast.ident;

    let base_field_name = Some(ast.data)
        .clone()
        .and_then(|data| match data {
            Data::Struct(data) => match data.fields {
                Fields::Named(fields) => Some(fields),
                _ => None,
            },
            _ => None,
        })
        .and_then(|fields| {
            fields.named.iter().find_map(|field| {
                field.attrs.iter().find_map(|attr| {
                    if attr.path().is_ident("base") {
                        Some(field.ident.clone().unwrap())
                    } else {
                        None
                    }
                })
            })
        })
        .unwrap_or_else(|| syn::Ident::new("node_base", Span::call_site()));

    let gen = quote! {
        impl NodeBaseTrait for #name {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }

            #[inline]
            fn base(&self) -> &NodeBase {
                &self.#base_field_name
            }

            #[inline]
            fn base_mut(&mut self) -> &mut NodeBase {
                &mut self.#base_field_name
            }
        }
    };

    gen.into()
}
