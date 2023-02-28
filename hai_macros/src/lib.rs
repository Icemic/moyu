mod node;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn node(args: TokenStream, struct_body: TokenStream) -> TokenStream {
    node::node(args, struct_body)
}
