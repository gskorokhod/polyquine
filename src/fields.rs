use crate::util::{Attrs, expand_sequence};
use itertools::Itertools;
use proc_macro2::{Ident, Punct, Spacing, TokenStream};
use quote::{ToTokens, TokenStreamExt, format_ident, quote};
use std::default::Default;
use syn::{
    AngleBracketedGenericArguments, Field, Fields, GenericArgument, Index, Path, PathArguments,
    PathSegment, Type,
};

/// Types that don't implement `ToTokens` in the way that we need, so require special handling.
pub enum TypeWrapper {
    Box(Type),
    Option(Type),
    Iterable(Type, Path),           // e.g. Vec<T>, HashSet<T>, VecDeque<T>, ...
    MapLike(Type, Type, Path),      // e.g. HashMap<K, V>
    ArcLike(Type, Path),            // e.g. Arc<T>, Rc<T>
    CellLike(Type, Path),           // e.g. Cell<T>, RefCell<T>
    OtherPathZero(Path),            // e.g. Path with no generics, like `std::string::String`
    OtherPathOne(Type, Path),       // e.g. Path with one generic, like `SomeTrait<T>`
    OtherPathMany(Vec<Type>, Path), // e.g. Path with multiple generics, like `SomeTrait<T, U, V>`
    Tuple(Vec<Type>),
}

static SUPPORTED_ITERABLES: &[&str] = &["Vec", "HashSet", "VecDeque", "BTreeSet"];
static SUPPORTED_MAPLIKES: &[&str] = &["HashMap", "BTreeMap"];
static SUPPORTED_CELLLIKE: &[&str] = &["Cell", "RefCell"];
static SUPPORTED_ARCLIKE: &[&str] = &["Arc", "Rc"];

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
            TypeWrapper::Box(inner)
            | TypeWrapper::Option(inner)
            | TypeWrapper::Iterable(inner, _) => leaf_types_impl(inner),
            TypeWrapper::MapLike(k, v, _) => {
                // For map-like types, we return both key and value types.
                let mut ans = leaf_types_impl(k);
                ans.extend(leaf_types_impl(v));
                ans
            }
            TypeWrapper::Tuple(inner) => inner.iter().flat_map(leaf_types_impl).collect(),
            TypeWrapper::CellLike(inner, _) => leaf_types_impl(inner),
            TypeWrapper::ArcLike(inner, _) => leaf_types_impl(inner),
            TypeWrapper::OtherPathZero(_) => vec![ty.clone()],
            TypeWrapper::OtherPathOne(inner, _) => vec![inner.clone()],
            TypeWrapper::OtherPathMany(inners, _) => {
                inners.iter().flat_map(leaf_types_impl).collect()
            }
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
            let mut path = path.clone();
            let segments = &mut path.path.segments;
            let last = segments.last_mut().unwrap().clone();
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
            let new_last = PathSegment {
                ident: last.ident.clone(),
                arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                    colon2_token: Some(Default::default()),
                    lt_token: Default::default(),
                    args: inners
                        .iter()
                        .map(|a| GenericArgument::Type(a.clone()))
                        .collect(),
                    gt_token: Default::default(),
                }),
            };
            segments.pop();
            segments.push(new_last);
            match ident.as_str() {
                "Option" => Some(TypeWrapper::Option(inners[0].clone())),
                "Box" => Some(TypeWrapper::Box(inners[0].clone())),
                _itr if SUPPORTED_ITERABLES.contains(&ident.as_str()) => {
                    // Handle types like `Vec`, `HashSet`, `VecDeque`, etc.
                    Some(TypeWrapper::Iterable(inners[0].clone(), path.path.clone()))
                }
                _map if SUPPORTED_MAPLIKES.contains(&ident.as_str()) => {
                    // Handle types like `HashMap`, `BTreeMap`, etc.
                    if inners.len() == 2 {
                        Some(TypeWrapper::MapLike(
                            inners[0].clone(),
                            inners[1].clone(),
                            path.path.clone(),
                        ))
                    } else {
                        println!("Invalid map-like type: {:#?}", ty);
                        None
                    }
                }
                _cell if SUPPORTED_CELLLIKE.contains(&ident.as_str()) => {
                    // Handle types like `Arc`, `Cell`, `RefCell`, etc.
                    if inners.len() == 1 {
                        Some(TypeWrapper::CellLike(inners[0].clone(), path.path.clone()))
                    } else {
                        println!("Invalid cell-like type: {:#?}", ty);
                        None
                    }
                }
                _arc if SUPPORTED_ARCLIKE.contains(&ident.as_str()) => {
                    // Handle types like `Arc`, `Rc`, etc.
                    if inners.len() == 1 {
                        Some(TypeWrapper::ArcLike(inners[0].clone(), path.path.clone()))
                    } else {
                        println!("Invalid arc-like type: {:#?}", ty);
                        None
                    }
                }
                _ => match inners.len() {
                    0 => Some(TypeWrapper::OtherPathZero(path.path.clone())),
                    1 => Some(TypeWrapper::OtherPathOne(
                        inners[0].clone(),
                        path.path.clone(),
                    )),
                    _ => Some(TypeWrapper::OtherPathMany(inners, path.path.clone())),
                },
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
pub fn expand_field(ty: &Type, ident: &Ident) -> (TokenStream, TokenStream) {
    match type_wrapper(ty) {
        Some(TypeWrapper::Box(inner)) => {
            let (inner_exp, inner_top) = expand_field(&inner, ident);
            let exp = quote! {
                Box::new(#inner_exp)
            };
            (exp, inner_top)
        }
        Some(TypeWrapper::Iterable(inner, path)) => {
            // Handle types like `HashSet`, `VecDeque`, etc.
            let inner_exp_ident = format_ident!("{}_vec_like", ident);
            let inner_item_ident = format_ident!("{}_vec_like_item", ident);
            let (inner_exp, inner_top) = expand_field(&inner, &inner_item_ident);
            let top = quote! {
                let #inner_exp_ident = #ident.iter().map(|#inner_item_ident: &#inner| {
                    #inner_top
                    ::quote::quote! { #inner_exp }
                }).collect::<Vec<_>>();
            };
            // Now, generate the `#path` containing the expanded items.
            let seq_exp = expand_sequence(&inner_exp_ident);
            let exp = if path.segments.last().unwrap().ident.to_string() == "Vec" {
                quote! { vec![#seq_exp] }
            } else {
                quote! {
                    #path::from([#seq_exp])
                }
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
            let mut top = TokenStream::new();
            let mut expansions = Vec::<TokenStream>::new();
            // First, assign idents to each field in the tuple.
            for i in 0..inner.len() {
                let field_ident = format_ident!("{}_tuple_{}", ident, i);
                let idx = Index::from(i);
                top.extend(quote! {
                    let #field_ident = &#ident.#idx;
                });
            }
            // Next, generate expansions for each field in the tuple.
            for (i, ty) in inner.iter().enumerate() {
                let field_ident = format_ident!("{}_tuple_{}", ident, i);
                let (exp, top_expr) = expand_field(ty, &field_ident);
                top.extend(top_expr.clone());
                expansions.push(exp);
            }
            // Finally, create the top-level expression that combines all expansions.
            let exp = quote! {
                (#(#expansions),*)
            };
            (exp, top)
        }
        Some(TypeWrapper::CellLike(inner, path)) => {
            // Handle types like `Cell<T>`, `RefCell<T>`, etc.
            println!(
                "WARNING: Tokenising a Cell-like type ({}) will effectively clone its value. This may not be the intended behaviour!",
                path.to_token_stream()
            );
            let inner_ident = format_ident!("{}_cell", ident);
            let (inner_exp, inner_top) = expand_field(&inner, &inner_ident);
            let top = quote! {
                #inner_top
                let #inner_ident = #ident.clone().into_inner();
            };
            let exp = quote! {
                #path::new(#inner_exp)
            };
            (exp, top)
        }
        Some(TypeWrapper::ArcLike(inner, path)) => {
            // Handle types like `Arc<T>`, `Rc<T>`, etc.
            println!(
                "WARNING: Arc-like type ({}) will be handled by cloning its inner value. This may not be the intended behaviour!",
                path.to_token_stream()
            );
            let inner_ident = format_ident!("{}_arc", ident);
            let (inner_exp, inner_top) = expand_field(&inner, &inner_ident);
            let top = quote! {
                #inner_top
                let #inner_ident = #ident.as_ref();
            };
            let exp = quote! {
                #path::new(#inner_exp)
            };
            (exp, top)
        }
        Some(TypeWrapper::MapLike(kt, vt, path)) => {
            let inner_ident = format_ident!("{}_map", ident);
            let key_ident = format_ident!("{}_map_key", ident);
            let value_ident = format_ident!("{}_map_value", ident);
            let (key_exp, key_top) = expand_field(&kt, &key_ident);
            let (value_exp, value_top) = expand_field(&vt, &value_ident);
            // Generate top-level binding
            let top = quote! {
                #key_top
                #value_top
                let #inner_ident = #ident.iter().map(|(#key_ident, #value_ident)| {
                    ::quote::quote! { (#key_exp, #value_exp) }
                }).collect::<Vec<_>>();
            };
            // Generate the key and value sequences.
            let seq = expand_sequence(&inner_ident);
            let exp = quote! {
                #path::from([#seq])
            };
            (exp, top)
        }
        _ => {
            // For other types, push `#a` (where `a` is the field identifier).
            // I.e. bind the field directly and use its own ToTokens implementation.
            let mut ts = TokenStream::new();
            ts.append(Punct::new('#', Spacing::Joint));
            ts.append(ident.clone());
            (quote! { #ts.into() }, TokenStream::new())
        }
    }
}
