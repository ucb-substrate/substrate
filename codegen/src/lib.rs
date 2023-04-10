use derive_interface::derive_interface_inner;
use hard_macro::hard_macro_inner;
use proc_macro::TokenStream;

mod derive_interface;
mod hard_macro;

#[proc_macro_attribute]
pub fn hard_macro(args: TokenStream, input: TokenStream) -> TokenStream {
    hard_macro_inner(args, input)
}

#[proc_macro_derive(Interface, attributes(input, output))]
pub fn derive_interface(item: TokenStream) -> TokenStream {
    derive_interface_inner(item)
}
