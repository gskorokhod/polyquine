//! Helpers that don't fit anywhere else.

use crate::fields::field_types;
use deluxe::ExtractAttributes;
use proc_macro2::{Delimiter, Group, Ident, Punct, Spacing, TokenStream};
use quote::TokenStreamExt;
use std::collections::HashSet;
use syn::punctuated::Punctuated;
use syn::{Data, DeriveInput, Error, Result, Token};
use syn::{ExprClosure, parse_quote};
use syn::{TypeParamBound, WhereClause, WherePredicate};

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

    #[deluxe(default = None)]
    pub custom_with: Option<ExprClosure>,
}

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

/// Expands to: "#(#ident),*"
pub fn expand_sequence(seq: &Ident) -> TokenStream {
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
