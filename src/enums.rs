use proc_macro2::{Ident, Punct, Spacing, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{Result, DataEnum, Fields, Variant, FieldsUnnamed};
use syn::spanned::Spanned;
use crate::util::{type_wrapper, TypeWrapper};

pub fn expand_enum(ty_name: &Ident, data: &DataEnum) -> Result<TokenStream> {
    let body = expand_variants(ty_name, data)?;

    Ok(quote! {
        match self {
            #body
        }
    })
}

fn expand_variants(ty_name: &Ident, data: &DataEnum) -> Result<TokenStream> {
    let arms: Vec<TokenStream> = data.variants.iter().map(|variant| {
        expand_variant(ty_name, variant)
    }).collect::<Result<Vec<_>>>()?;

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
        },
        Fields::Unnamed(fields) => {
            // For tuple variants, generate bindings for each field.
            // E.g: Enum::Tuple(a, b) -> quote! { Enum::Tuple(#a, #b) }
            let mut bindings: Vec<TokenStream> = vec![];
            let mut expansions: Vec<TokenStream> = vec![];
            for (i, field) in fields.unnamed.iter().enumerate() {
                let field_ident = Ident::new(&format!("field_{}", i), field.span());
                bindings.push(field_ident.to_token_stream());
                match type_wrapper(&field.ty) {
                    Some(TypeWrapper::Box(inner)) => {
                        expansions.push(quote! {
                            Box::new(#field_ident.into()) // TODO: Handler should be recursive (e.g. Box<Box<T>>)
                        })
                    }
                    Some(TypeWrapper::Vec(inner)) => {
                        let hash = Punct::new('#', Spacing::Joint);
                        expansions.push(quote! { vec![ #hash (#hash #field_ident.into(),)* ] })
                    }
                    Some(TypeWrapper::Option(inner)) => {
                        let mut value = TokenStream::new();
                        value.append(Punct::new('#', Spacing::Joint));
                        value.append(Ident::new("value", field.span()));

                        expansions.push(quote! {
                            match #field_ident {
                                Some(value) => quote! { Some(#value.into()) }, // TODO: Handler should be recursive (e.g. Option<Option<T>>)
                                None => quote! { None },
                            } // TODO: Handler should be recursive (e.g. Option<Option<T>>)
                        });
                    }
                    Some(TypeWrapper::Tuple(inner)) => {
                        todo!("Handle tuple types in enum variants");
                    }
                    None => {
                        // For other types, push `#a` (where `a` is the field identifier).
                        // I.e. bind the field directly and use its own ToTokens implementation.
                        let mut ts = TokenStream::new();
                        ts.append(Punct::new('#', Spacing::Joint));
                        ts.append(field_ident);
                        expansions.push(quote! {
                            #ts.into()
                        });
                    }
                }
            }
            Ok(quote! {
                #prefix::#variant_name ( #( #bindings ),* ) => {
                    tokens.extend(::quote::quote! { #prefix::#variant_name ( #(#expansions),* ) });
                }
            })
        },
        Fields::Named(fields) => {
            // For named variants, generate bindings for each field using their names.
            // E.g: Enum::Named { a, b } -> quote! { Enum::Named { a: #a, b: #b } }
            todo!("Handle named fields in enums");
        },
    }
}
