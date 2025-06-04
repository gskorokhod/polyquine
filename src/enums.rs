use proc_macro2::{Delimiter, Group, Ident, Punct, Spacing, TokenStream};
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::{Result, DataEnum, Fields, Variant, FieldsUnnamed, Type};
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

            let mut bindings: Vec<TokenStream> = vec![];   // Field bindings (e.g. `match Enum::Tuple(a, b)`)
            let mut expansions: Vec<TokenStream> = vec![]; // Expanded field expressions (e.g. `#a`, `#b`)
            let mut top_exprs: Vec<TokenStream> = vec![];  // Top-level expressions for each field (e.g. `let a = &self.a;`)

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
        },
        Fields::Named(fields) => {
            // For named variants, generate bindings for each field using their names.
            // E.g: Enum::Named { a, b } -> quote! { Enum::Named { a: #a, b: #b } }
            todo!("Handle named fields in enums");
        },
    }
}

fn expand_field(ty: &Type, ident: &Ident) -> (TokenStream, TokenStream) {
    match type_wrapper(ty) {
        Some(TypeWrapper::Box(inner)) => {
            let (inner_exp, inner_top) = expand_field(&inner, ident);
            let exp = quote! {
                Box::new(#inner_exp)
            };
            (exp, inner_top)
        }
        Some(TypeWrapper::Vec(inner)) => {
            // Expand the inner type and save its generated `quote!` expression.
            let inner_exp_ident = format_ident!("{}_vec", ident);
            let inner_item_ident = format_ident!("{}_vec_item", ident);
            let (inner_exp, inner_top) = expand_field(&inner, &inner_item_ident);
            let top = quote! {
                let #inner_exp_ident = #ident.iter().map(|#inner_item_ident: &#inner| {
                    #inner_top
                    ::quote::quote! { #inner_exp }
                }).collect::<Vec<_>>();
            };
            // Now, generate the `vec!` containing the expanded items.
            let seq_exp = expand_sequence(&inner_exp_ident);
            let exp = quote! {
                vec![ #seq_exp ]
            };
            (exp, top)
        }
        Some(TypeWrapper::Option(inner)) => {
            let gen_ident = format_ident!("{}_option", ident);
            let val_ident = format_ident!("{}_option_val", ident);
            // Expand the inner type and save its generated `quote!` expression.
            let (inner_val, inner_top) = expand_field(&inner, &val_ident);
            // Generate a match! statement that handles the `Option` type.
            let top = quote! {
                #inner_top
                let #gen_ident = match #ident {
                    Some(#val_ident) => ::quote::quote! { Some(#inner_val) },
                    None => ::quote::quote! { None },
                };
            };
            let mut exp = TokenStream::new();
            exp.append(Punct::new('#', Spacing::Joint));
            exp.append(gen_ident);
            (exp, top)
        }
        Some(TypeWrapper::Tuple(inner)) => {
            todo!("Handle tuple types in enum variants");
        }
        None => {
            // For other types, push `#a` (where `a` is the field identifier).
            // I.e. bind the field directly and use its own ToTokens implementation.
            let mut ts = TokenStream::new();
            ts.append(Punct::new('#', Spacing::Joint));
            ts.append(ident.clone());
            (quote!{ #ts.into() }, TokenStream::new())
        }
    }
}

fn expand_sequence(seq: &Ident) -> TokenStream {
    // Expands to: "#(#ident),*"
    let mut inner = TokenStream::new();

    // Add "#"
    inner.append(Punct::new('#', Spacing::Alone));

    // Add parenthesized group "(#<field ident>)"
    let mut paren_content = TokenStream::new();
    paren_content.append(Punct::new('#', Spacing::Alone));
    paren_content.append(seq.clone());
    let paren_group = Group::new(Delimiter::Parenthesis, paren_content);
    inner.append(paren_group);

    // Add ",*"
    inner.append(Punct::new(',', Spacing::Alone));
    inner.append(Punct::new('*', Spacing::Alone));

    inner
}
