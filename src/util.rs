//! Helpers that don't fit anywhere else.

use deluxe::ExtractAttributes;
use itertools::Itertools;
use std::collections::HashSet;
use syn::punctuated::Punctuated;
use syn::{parse_quote, GenericArgument, PathArguments};
use syn::{Data, DeriveInput, Error, Field, Fields, Result, Token, Type};
use syn::{TypeParamBound, WhereClause, WherePredicate};

/// Generates a `where` clause with the specified bounds applied to all field types.
pub fn add_bounds(
    input: DeriveInput,
    where_clause: Option<&WhereClause>,
    bounds: Punctuated<TypeParamBound, Token![+]>,
) -> Result<WhereClause> {
    let unique_types: HashSet<_> = match input.data {
        Data::Union(_) => return Err(Error::new_spanned(input, "unions are not supported")),
        Data::Struct(data) => HashSet::from_iter(field_types(data.fields)),
        Data::Enum(data) => data
            .variants
            .into_iter()
            .flat_map(|v| field_types(v.fields))
            .collect::<HashSet<_>>(),
    };

    let mut where_clause = where_clause.cloned().unwrap_or_else(|| WhereClause {
        where_token: Default::default(),
        predicates: Default::default(),
    });

    where_clause
        .predicates
        .extend(unique_types.iter().map(|ty| -> WherePredicate {
            parse_quote! {
                #ty: #bounds
            }
        }));

    Ok(where_clause)
}

/// Helper type for parsing the meta attributes of the
/// type for which `Parse` and `ToTokens` are being `#[derive]`d.
#[derive(Clone, Default, Debug, ExtractAttributes)]
#[deluxe(attributes(polyquine))]
pub struct Attrs {
    /// Indicates that the field participates in (possibly mutual) recursion
    /// at the type level, e.g. a parent-child relationship within the same
    /// struct/enum. The type of such fields will be omitted from the `where`
    /// clause in the derived implementations, because the corresponding
    /// constraints can't be satisfied, and the code compiles without them.
    ///
    /// Hopefully, this can be removed in the future once Chalk lands;
    /// see [this issue](https://github.com/rust-lang/rust/issues/48214)
    #[deluxe(default = false)]
    pub recursive: bool,
}

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
        Fields::Named(fields) => fields
            .named
            .into_iter()
            .flat_map(leaf_types)
            .collect_vec(),
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
