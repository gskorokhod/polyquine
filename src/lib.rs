mod enums;
mod fields;
mod structs;
mod util;

use crate::util::add_bounds;
use enums::expand_enum;
use proc_macro2::TokenStream;
use quote::quote;
use structs::expand_struct;
use syn::parse_quote;
use syn::{Data, DeriveInput, Error, Result};

#[proc_macro_derive(ToTokens, attributes(polyquine))]
pub fn derive_to_tokens(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let res = expand(item.into()).unwrap_or_else(|err| err.to_compile_error());
    res.into()
}

/// Actual implementation of `#[derive(ToTokens)]`.
fn expand(stream: TokenStream) -> Result<TokenStream> {
    let item: DeriveInput = syn::parse2(stream)?;
    let ty_name = item.ident.clone();

    let body = match &item.data {
        Data::Union(_) => return Err(Error::new_spanned(&item, "unions are not supported")),
        Data::Enum(data) => expand_enum(&ty_name, data)?,
        Data::Struct(data) => expand_struct(&ty_name, data)?,
    };
    let generics = item.generics.clone();
    let (impl_gen, ty_gen, where_clause) = generics.split_for_impl();
    let bounds = parse_quote!(::quote::ToTokens);
    let where_clause = add_bounds(item, where_clause, bounds)?;

    Ok(quote! {
        #[automatically_derived]
        #[allow(non_snake_case)]
        impl #impl_gen ::quote::ToTokens for #ty_name #ty_gen #where_clause {
            fn to_tokens(&self, tokens: &mut ::proc_macro2::TokenStream) {
                #body
            }
        }
    })
}
