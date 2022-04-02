extern crate proc_macro;

// using proc_macro_attribute to declare an attribute like procedural macro
use proc_macro::TokenStream;

use ::proc_macro2::{Span, TokenStream as TokenStream2};
use ::quote::{quote, quote_spanned};
use ::syn::Result;
// use ::syn::*;
use convert_case::{Case, Casing};
use proc_macro2::Ident;
use syn::{
    parse_macro_input, token, Data, DataEnum, DataStruct, DataUnion, DeriveInput, Error, Fields,
    LitStr,
};

#[proc_macro_derive(RouterOsApiFieldAccess)]
pub fn my_trait_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as _);
    TokenStream::from(match impl_my_trait(ast) {
        Ok(it) => it,
        Err(err) => err.to_compile_error(),
    })
}
fn impl_my_trait(ast: DeriveInput) -> Result<TokenStream2> {
    Ok({
        let name = ast.ident;
        let fields = match ast.data {
            Data::Enum(DataEnum {
                enum_token: token::Enum { span },
                ..
            })
            | Data::Union(DataUnion {
                union_token: token::Union { span },
                ..
            }) => {
                return Err(Error::new(span, "Expected a `struct`"));
            }

            Data::Struct(DataStruct {
                fields: Fields::Named(it),
                ..
            }) => it,

            Data::Struct(_) => {
                return Err(Error::new(
                    Span::call_site(),
                    "Expected a `struct` with named fields",
                ));
            }
        };

        let data_expanded_mut_members = fields.named.clone().into_iter().map(|field| {
            let field_name = field.ident.expect("Unreachable");
            let span = field_name.span();
            let field_name_stringified = LitStr::new(&field2ros(&field_name), span);
            let ret = quote_spanned! { span=>
                (#field_name_stringified, &mut self.#field_name)
            };
            //println!("quote_spanned! {field_name}: {}", ret.to_string());
            ret
        }); // : impl Iterator<Item = TokenStream2>
        let data_expanded_members = fields.named.clone().into_iter().map(|field| {
            let field_name = field.ident.expect("Unreachable");
            let span = field_name.span();
            let field_name_stringified = LitStr::new(&field2ros(&field_name), span);
            quote_spanned! { span=>
                (#field_name_stringified, &self.#field_name)
            }
        });
        let x = quote! {
            impl crate::routeros::model::RouterOsApiFieldAccess for #name {
                fn fields_mut(&mut self) -> Box<dyn Iterator<Item = (&str, &mut dyn RosFieldAccessor)> + '_>
                {
                    let fields: Vec<(&str, &mut dyn RosFieldAccessor)> = vec![
                       #(#data_expanded_mut_members ,)*
                    ];
                    Box::new(fields.into_iter())
                }
                fn fields(&self) -> Box<dyn Iterator<Item = (&str, &dyn RosFieldAccessor)> + '_>{
                    let fields: Vec<(&str, &dyn RosFieldAccessor)> = vec![
                        #(#data_expanded_members ,)*
                    ];
                    Box::new(fields.into_iter())

                }
            }
        };
        //println!("code: {}", x);
        x
    })
}

fn field2ros(field_name: &Ident) -> String {
    match field_name.to_string().as_str() {
        "id" => String::from(".id"),
        "nextid" => String::from(".nextid"),
        name => name.to_case(Case::Kebab),
    }
}
