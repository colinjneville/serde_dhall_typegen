mod push_cd;
mod single;

use proc_macro2::{TokenStream, Span};
use quote::{quote, quote_spanned};

pub use push_cd::*;
pub use single::*;

pub fn create_error(msg: &str) -> TokenStream {
    quote!(std::compile_error!(#msg);)
}

pub fn create_spanned_error(msg: &str, span: Span) -> TokenStream {
    quote_spanned!(span => std::compile_error!(#msg);)
}