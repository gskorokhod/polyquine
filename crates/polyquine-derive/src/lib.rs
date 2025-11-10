use proc_macro::TokenStream;
use proc_macro2::{Punct, Spacing, TokenStream as TokenStream2};
use quote::{TokenStreamExt, quote, ToTokens};
use syn::parse::ParseStream;
use syn::{
    Data, DeriveInput, Fields, Generics, Ident, Index, Path, WhereClause, WherePredicate,
    spanned::Spanned,
};

fn hash_ident(ident: &Ident) -> TokenStream2 {
    let mut hash_field = TokenStream2::new();
    hash_field.append(Punct::new('#', Spacing::Joint));
    hash_field.append(ident.clone());
    hash_field
}

fn build_path_setup(ident: &Ident, module_prefix: Option<&Path>) -> TokenStream2 {
    match module_prefix {
        Some(prefix) => {
            quote! {
                let path: ::syn::Path = ::syn::parse_quote!(#prefix::#ident);
            }
        }
        None => {
            quote! {
                let fully_qualified_path = concat!(module_path!(), "::", stringify!(#ident));
                let path: ::syn::Path = ::syn::parse_str(fully_qualified_path).unwrap();
            }
        }
    }
}

/// When deriving for types with generics, we add an extra bound `T: ... + Quine`.
/// For example, given:
/// ```ignore
/// struct MyStruct<T: SomeTrait> {
///     field: T,
/// }
/// ```
/// We want to generate:
/// ```ignore
/// impl<T: SomeTrait + Quine> Quine for MyStruct<T> {
///     ...
/// }
/// ```
/// To do that, we either modify the existing where clause or create a new one.
fn build_where_clause(generics: &Generics) -> Option<WhereClause> {
    let gen_params = generics.params.iter().collect::<Vec<_>>();
    if gen_params.is_empty() {
        return None;
    }
    match &generics.where_clause {
        Some(a_wc) => {
            let mut wc = a_wc.clone();
            for p in &mut wc.predicates {
                if let WherePredicate::Type(pt) = p {
                    let tb: syn::TraitBound = syn::parse_str("Quine").unwrap();
                    (&mut pt.bounds).push(syn::TypeParamBound::Trait(tb));
                }
            }
            Some(wc)
        }
        None => {
            let mut bounds: Vec<syn::TypeParam> = Vec::new();
            for p in &gen_params {
                if let syn::GenericParam::Type(a_tp) = p {
                    let mut tp = a_tp.clone();
                    tp.default = None;
                    let tb = syn::parse_str("Quine").unwrap();
                    (&mut tp.bounds).push(syn::TypeParamBound::Trait(tb));
                    bounds.push(tp);
                }
            }
            let where_toks = quote! {
                where #(#bounds),*
            };
            let wc = syn::parse2(where_toks.clone()).expect(format!("Could not parse where_toks: `{}`", where_toks).as_str());
            Some(wc)
        }
    }
}

fn parse_custom_arm(variant: &syn::Variant) -> Option<TokenStream2> {
    for attr in &variant.attrs {
        if attr.path().is_ident("polyquine_with") {
            return attr
                .parse_args_with(|parser: ParseStream| {
                    // Parse "arm ="
                    let ident: syn::Ident = parser.parse()?;
                    if ident != "arm" {
                        return Err(parser.error("expected 'arm ='"));
                    }

                    let _: syn::Token![=] = parser.parse()?;

                    // Parse everything after "arm =" as tokens
                    let mut tokens = TokenStream2::new();
                    while !parser.is_empty() {
                        tokens.extend(TokenStream2::from(
                            parser.parse::<proc_macro2::TokenTree>()?,
                        ));
                    }

                    Ok(tokens)
                })
                .ok();
        }
    }
    None
}

#[proc_macro_derive(Quine, attributes(path_prefix, polyquine_skip, polyquine_with))]
pub fn derive_quine(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse2(input.clone().into()).expect(format!("Could not parse macro input: {input}").as_str());
    let mut generics = input.generics;
    let ident = input.ident;

    // Add "T: ... + Quine" to the where clause for types with generics
    generics.where_clause = build_where_clause(&generics);

    // Parse the path_prefix attribute, if any
    let mut module_prefix: Option<Path> = None;
    for attr in &input.attrs {
        if attr.path().is_ident("path_prefix") {
            if module_prefix.is_some() {
                return syn::Error::new(attr.span(), "duplicate path_prefix attribute")
                    .to_compile_error()
                    .into();
            }
            match attr.parse_args::<Path>() {
                Ok(path) => module_prefix = Some(path),
                Err(err) => return err.to_compile_error().into(),
            }
        }
    }

    let path_setup = build_path_setup(&ident, module_prefix.as_ref());
    let (impl_gen, ty_gen, where_clause) = generics.split_for_impl();

    let body =
        match input.data {
            // Derive for structs
            Data::Struct(data) => match &data.fields {
                Fields::Unit => {
                    let path = hash_ident(&Ident::new(&"path", proc_macro2::Span::call_site()));
                    quote! {
                        #path_setup
                        ::quote::quote!{#path {}}
                    }
                }
                Fields::Unnamed(fields) => {
                    let (decls, exps): (Vec<TokenStream2>, Vec<TokenStream2>) = fields
                        .unnamed
                        .iter()
                        .enumerate()
                        .map(|(i, f)| {
                            let idnt = Ident::new(format!("gen_field_{}", i).as_str(), f.span());
                            let idx = Index::from(i);
                            let field_let = quote! {
                                let #idnt = self.#idx.ctor_tokens();
                            };
                            (field_let, hash_ident(&idnt))
                        })
                        .unzip();
                    let path = hash_ident(&Ident::new(&"path", proc_macro2::Span::call_site()));
                    quote! {
                        #(#decls)*
                        #path_setup
                        ::quote::quote!{#path(#(#exps),*)}
                    }
                }
                Fields::Named(fields) => {
                    let (decls, exps): (Vec<TokenStream2>, Vec<TokenStream2>) = fields
                        .named
                        .iter()
                        .map(|f| {
                            let f_toks = f.to_token_stream().to_string();
                            let ident = f.ident.as_ref().expect(format!("Could not get ident of named struct field {f_toks}").as_str());
                            let field_let = quote! {
                                let #ident = self.#ident.ctor_tokens();
                            };
                            let hash_ident = hash_ident(ident);
                            let field_exp = quote! {#ident: #hash_ident};
                            (field_let, field_exp)
                        })
                        .unzip();
                    let path = hash_ident(&Ident::new(&"path", proc_macro2::Span::call_site()));
                    quote! {
                        #(#decls)*
                        #path_setup
                        ::quote::quote!{#path{#(#exps),*}}
                    }
                }
            },
            Data::Enum(data) => {
                // Derive for enums
                let arms = data.variants.iter().map(|v| {
                let variant_ident = &v.ident;

                // Skipped enum variants
                let is_skipped = v.attrs.iter().any(|attr| attr.path().is_ident("polyquine_skip"));
                if is_skipped {
                    let skipped_msg = quote! {
                        {
                            panic!("Attempted to call ctor_tokens() on skipped enum variant {}::{}",
                                   stringify!(#ident), stringify!(#variant_ident))
                        }
                    };
                   return match &v.fields {
                        Fields::Unit => quote! {#ident::#variant_ident => #skipped_msg},
                        Fields::Unnamed(_) => quote! {#ident::#variant_ident(..) => #skipped_msg},
                        Fields::Named(_) => quote! {#ident::#variant_ident{..} => #skipped_msg},
                    };
                }

                // Custom arm for enum variant
                if let Some(custom_arm) = parse_custom_arm(v) {
                    return quote! {#ident::#variant_ident #custom_arm};
                };

                match &v.fields {
                    Fields::Unit => {
                        let path = hash_ident(&Ident::new(&"path", proc_macro2::Span::call_site()));
                        quote! {#ident::#variant_ident => {
                            #path_setup
                            ::quote::quote!{#path::#variant_ident}
                        }}
                    }
                    Fields::Unnamed(fields) => {
                        let mut binds: Vec<Ident> = Vec::new();
                        let mut decls: Vec<TokenStream2> = Vec::new();
                        let mut exps: Vec<TokenStream2> = Vec::new();
                        for (i, f) in fields.unnamed.iter().enumerate() {
                            let ident = Ident::new(format!("gen_field_{}", i).as_str(), f.span());
                            let exp_ident =
                                Ident::new(format!("gen_field_{}_exp", i).as_str(), f.span());
                            let field_let = quote! {
                                let #exp_ident = #ident.ctor_tokens();
                            };
                            binds.push(ident);
                            decls.push(field_let);
                            exps.push(hash_ident(&exp_ident));
                        }
                        let path = hash_ident(&Ident::new(&"path", proc_macro2::Span::call_site()));
                        quote! {
                            #ident::#variant_ident(#(#binds),*) => {
                                #(#decls)*
                                #path_setup
                                ::quote::quote!{#path::#variant_ident(#(#exps),*)}
                            }
                        }
                    }
                    Fields::Named(fields) => {
                        let mut binds: Vec<Ident> = Vec::new();
                        let mut decls: Vec<TokenStream2> = Vec::new();
                        let mut exps: Vec<TokenStream2> = Vec::new();
                        for f in fields.named.iter() {
                            let f_toks = f.to_token_stream().to_string();
                            let ident = f.ident.as_ref().expect(format!("Could not get ident of named enum field {f_toks}").as_str());
                            let exp_ident =
                                Ident::new(format!("gen_field_{}_exp", ident).as_str(), f.span());
                            let field_let = quote! {
                                let #exp_ident = #ident.ctor_tokens();
                            };
                            let hash_ident = hash_ident(&exp_ident);

                            binds.push(ident.clone());
                            decls.push(field_let);
                            exps.push(quote! { #ident: #hash_ident });
                        }
                        let path = hash_ident(&Ident::new(&"path", proc_macro2::Span::call_site()));
                        quote! {
                            #ident::#variant_ident{#(#binds),*} => {
                                #(#decls)*
                                #path_setup
                                ::quote::quote!{#path::#variant_ident{#(#exps),*}}
                            }
                        }
                    }
                }
            });
                quote! {
                    match self {
                        #(#arms),*
                    }
                }
            }
            Data::Union(_) => {
                unimplemented!("Unions are not supported")
            }
        };

    let ans = quote! {
        impl #impl_gen Quine for #ident #ty_gen #where_clause {
            fn ctor_tokens(&self) -> ::proc_macro2::TokenStream {
                #body
            }
        }
    };
    ans.into()
}
