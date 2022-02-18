extern crate proc_macro;

// using proc_macro_attribute to declare an attribute like procedural macro
use proc_macro::TokenStream;
use quote::{quote, ToTokens};

use syn::token::Token;
use syn::{parse_macro_input, ItemStruct};

#[proc_macro_attribute]
// _metadata is argument provided to macro call and _input is code to which attribute like macro attaches
pub fn ros_struct(attr: TokenStream, input: TokenStream) -> TokenStream {
    let input_struct: ItemStruct = parse_macro_input!(input as ItemStruct);

    // returning a simple TokenStream for Struct
    let name = attr.to_string();
    println!("Name: {}", name);

    let stream = TokenStream::from(quote! {
        struct Testing{
            world: String,
        }
    });
    //input_struct.to_token_stream().into()
    input_struct.to_token_stream().into()
}
