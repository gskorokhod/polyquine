use crate::fields::expand_field;
use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, quote};
use syn::spanned::Spanned;
use syn::{DataEnum, Fields, Result, Variant};

pub fn expand_enum(ty_name: &Ident, data: &DataEnum) -> Result<TokenStream> {
    let body = expand_variants(ty_name, data)?;

    Ok(quote! {
        match self {
            #body
        }
    })
}

fn expand_variants(ty_name: &Ident, data: &DataEnum) -> Result<TokenStream> {
    let arms: Vec<TokenStream> = data
        .variants
        .iter()
        .map(|variant| expand_variant(ty_name, variant))
        .collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        #(#arms)*
    })
}

fn expand_variant(prefix: &impl ToTokens, variant: &Variant) -> Result<TokenStream> {
    let variant_name = &variant.ident;
    match &variant.fields {
        Fields::Unit => {
            // For unit variants, there are no fields to bind.
            // E.g: Enum::Unit -> quote! { Enum::Unit }
            Ok(quote! {
                #prefix::#variant_name => {
                    tokens.extend(::quote::quote! { #prefix::#variant_name });
                }
            })
        }
        Fields::Named(_) | Fields::Unnamed(_) => {
            let mut bindings: Vec<TokenStream> = vec![]; // Field bindings (e.g. `match Enum::Tuple(a, b)`)
            let mut expansions: Vec<TokenStream> = vec![]; // Expanded field expressions (e.g. `#a`, `#b`)
            let mut top_exprs: Vec<TokenStream> = vec![]; // Top-level expressions for each field (e.g. `let a = &self.a;`)

            match &variant.fields {
                Fields::Unnamed(fields) => {
                    // For tuple variants, generate bindings for each field.
                    // E.g: Enum::Tuple(a, b) -> quote! { Enum::Tuple(#a, #b) }
                    for (i, field) in fields.unnamed.iter().enumerate() {
                        let field_ident = Ident::new(&format!("field_{}", i), field.span());
                        let (exp, top) = expand_field(&field.ty, &field_ident);
                        bindings.push(field_ident.to_token_stream());
                        expansions.push(exp);
                        top_exprs.push(top);
                    }
                    Ok(quote! {
                        #prefix::#variant_name ( #( #bindings ),* ) => {
                            #(#top_exprs)*
                            tokens.extend(::quote::quote! { #prefix::#variant_name ( #(#expansions),* ) });
                        }
                    })
                }
                Fields::Named(fields) => {
                    // For named variants, generate bindings for each field using their names.
                    // E.g: Enum::Named { a, b } -> quote! { Enum::Named { a: #a, b: #b } }
                    for field in &fields.named {
                        let field_ident = field
                            .ident
                            .as_ref()
                            .expect("named variant must have field names");
                        let (exp, top) = expand_field(&field.ty, field_ident);
                        bindings.push(field_ident.to_token_stream());
                        expansions.push(exp);
                        top_exprs.push(top);
                    }
                    Ok(quote! {
                        #prefix::#variant_name { #( #bindings ),* } => {
                            #(#top_exprs)*
                            tokens.extend(::quote::quote! { #prefix::#variant_name {
                                #( #bindings: #expansions ),*
                            } });
                        }
                    })
                }
                _ => unreachable!(),
            }
        }
    }
}
