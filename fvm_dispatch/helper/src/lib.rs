use proc_macro::TokenStream;

use convert_case::{Case, Casing};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Ident, LitStr, Result};

mod hash;
use crate::hash::{Blake2bHasher, MethodResolver};

enum MethodName {
    Ident(Ident),
    Text(LitStr),
}

impl MethodName {
    /// Hash the method name
    ///
    /// - Text (string) gets hashed as-is
    /// - Identifiers (function names) get converted to PascalCase to meet naming rules
    fn hash(&self) -> u64 {
        let resolver = MethodResolver::new(Blake2bHasher {});
        let method_name = match self {
            MethodName::Ident(i) => i.to_string().to_case(Case::Pascal),
            MethodName::Text(s) => s.value(),
        };

        resolver
            .method_number(&method_name)
            .expect("invalid method name")
    }
}

impl Parse for MethodName {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(LitStr) {
            input.parse().map(MethodName::Text)
        } else if lookahead.peek(Ident) {
            input.parse().map(MethodName::Ident)
        } else {
            Err(lookahead.error())
        }
    }
}

#[proc_macro]
pub fn method_hash(input: TokenStream) -> TokenStream {
    let name: MethodName = parse_macro_input!(input);
    let hash = name.hash() as u32;
    // output a u32 literal as our hashed value
    quote!(#hash).into()
}
