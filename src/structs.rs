use crate::fields::expand_field;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{DataStruct, Fields, Result};

pub fn expand_struct(ty_name: &Ident, data: &DataStruct) -> Result<TokenStream> {
    //todo!("Handling structs is not yet implemented");
    match &data.fields {
        Fields::Named(fields) => {
            let mut top = TokenStream::new();
            let mut exps: Vec<TokenStream> = vec![]; // Expanded field expressions (e.g. `a: #a_exp`)
            for field in fields.named.iter() {
                match field.ident {
                    Some(ref ident) => {
                        let (exp, top_expr) = expand_field(&field, ident);
                        exps.push(quote! { #ident: #exp });
                        top.extend(quote! {
                            let #ident = &self.#ident;
                            #top_expr
                        });
                    }
                    None => {
                        return Err(syn::Error::new(
                            field.span(),
                            "Named fields must have an identifier",
                        ));
                    }
                }
            }
            Ok(quote! {
                #top
                tokens.extend(::quote::quote! { #ty_name { #( #exps ),* } });
            })
        }
        Fields::Unnamed(fields) => {
            let mut top = TokenStream::new();
            let mut exps: Vec<TokenStream> = vec![]; // Expanded field expressions (e.g. `#a_exp`)
            for (i, field) in fields.unnamed.iter().enumerate() {
                let ident = Ident::new(&format!("field_{}", i), field.span());
                let idx = syn::Index::from(i);
                let (exp, top_expr) = expand_field(&field, &ident);
                exps.push(exp);
                top.extend(quote! {
                    let #ident = &self.#idx;
                    #top_expr
                });
            }
            Ok(quote! {
                #top
                tokens.extend(::quote::quote! { #ty_name ( #( #exps ),* ) });
            })
        }
        Fields::Unit => Ok(quote! {
            tokens.extend(::quote::quote! { #ty_name {} });
        }),
    }
}
