use crate::util::{Attrs, expand_sequence};
use itertools::Itertools;
use proc_macro2::{Ident, Punct, Spacing, TokenStream};
use quote::{TokenStreamExt, format_ident, quote};
use syn::{Field, Fields, GenericArgument, PathArguments, Type};

/// Types that don't implement `ToTokens` in the way that we need, so require special handling.
pub enum TypeWrapper {
    Box(Type),
    Vec(Type),
    Option(Type),
    Tuple(Vec<Type>),
}

/// Get the types of all fields in a struct or enum variant.
pub fn field_types(flds: Fields) -> Vec<Type> {
    match flds {
        Fields::Unit => vec![],
        Fields::Named(fields) => fields.named.into_iter().flat_map(leaf_types).collect_vec(),
        Fields::Unnamed(fields) => fields
            .unnamed
            .into_iter()
            .flat_map(leaf_types)
            .collect_vec(),
    }
}

/// Get the leaf types of a field;
/// E.g. if the field is a `Vec<Option<Box<(i32, bool)>>>`, it will return [i32, bool].
pub fn leaf_types(mut field: Field) -> Vec<Type> {
    let attrs: Attrs = deluxe::extract_attributes(&mut field).unwrap();
    if attrs.recursive {
        return vec![];
    }
    leaf_types_impl(&field.ty)
}

/// Recursive implementation of `leaf_types`.
pub fn leaf_types_impl(ty: &Type) -> Vec<Type> {
    match type_wrapper(ty) {
        Some(fw) => match &fw {
            TypeWrapper::Box(inner) | TypeWrapper::Vec(inner) | TypeWrapper::Option(inner) => {
                leaf_types_impl(inner)
            }
            TypeWrapper::Tuple(inner) => inner.iter().flat_map(leaf_types_impl).collect(),
        },
        None => vec![ty.clone()],
    }
}

/// Parse a type and return a `TypeWrapper` if it matches one of the special-cased types.
pub fn type_wrapper(ty: &Type) -> Option<TypeWrapper> {
    // println!("Field: {}", ty.into_token_stream().to_string());
    // println!("AST: {:#?}", ty);
    match ty {
        Type::Path(path) => {
            let last = path.path.segments.last().unwrap();
            let ident = last.ident.to_string();
            let inners = match last.arguments {
                PathArguments::AngleBracketed(ref args) => args
                    .args
                    .iter()
                    .filter_map(|a| match a {
                        GenericArgument::Type(inner) => Some(inner.clone()),
                        _ => {
                            println!("Couldn't parse generic type argument: {:#?}", a);
                            None
                        }
                    })
                    .collect_vec(),
                _ => {
                    // println!("Invalid type arguments for: {:#?}", ty);
                    vec![]
                }
            };
            match ident.as_str() {
                "Option" | "Box" | "Vec" => {
                    if inners.len() != 1 {
                        // println!("Invalid type arguments for: {:#?}", ty);
                        // println!("Expected 1, got: {:#?}", inners);
                        panic!("Expected 1, got: {:#?}", inners);
                    }
                    match ident.as_str() {
                        "Option" => Some(TypeWrapper::Option(inners[0].clone())),
                        "Box" => Some(TypeWrapper::Box(inners[0].clone())),
                        "Vec" => Some(TypeWrapper::Vec(inners[0].clone())),
                        _ => unreachable!(),
                    }
                }
                _ => None,
            }
        }
        Type::Tuple(inner) => {
            let mut tuple = Vec::new();
            for ty in inner.elems.iter() {
                tuple.push(ty.clone());
            }
            Some(TypeWrapper::Tuple(tuple))
        }
        _ => None,
    }
}

/// Generate the `quote!` expression for a field of a struct or enum variant.
///
/// ## Input
/// - `ty`: The type of the field.
/// - `ident`: The identifier of the field.
///
/// ## Output
/// Returns a tuple containing:
/// 1. The `quote!` expression for the field.
/// 2. The top-level expression needed to bind the field
///
/// ## Example
/// For a field of type `Option<Box<i32>>` with identifier `value`, it will return:
///
/// top:
/// ```none
/// let value_option = match value {
///     Some(val) => ::quote::quote! { Some(Box::new(#val)) },
///     None => ::quote::quote! { None },
///  };
/// ```
/// expansion:
/// ```none
/// ::quote::quote! { #value_option.into() }
/// ```
///
pub fn expand_field(ty: &Type, ident: &Ident) -> (TokenStream, TokenStream) {
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
            (quote! { #ts.into() }, TokenStream::new())
        }
    }
}
