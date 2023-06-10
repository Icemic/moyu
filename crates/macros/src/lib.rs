mod hai_bindgen;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn hai_bindgen(args: TokenStream, func_body: TokenStream) -> TokenStream {
    hai_bindgen::entry(args, func_body)
}
