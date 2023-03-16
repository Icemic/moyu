mod hai_bindgen;
mod node;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn node(args: TokenStream, struct_body: TokenStream) -> TokenStream {
    node::node(args, struct_body)
}

#[proc_macro_attribute]
pub fn hai_bindgen(args: TokenStream, func_body: TokenStream) -> TokenStream {
    hai_bindgen::entry(args, func_body)
}
