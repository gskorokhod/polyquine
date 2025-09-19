use proc_macro::TokenStream;
use proc_macro2::{Punct, Spacing, TokenStream as TokenStream2};
use quote::{TokenStreamExt, quote};
use syn::{Data, DeriveInput, Fields, Ident, Index, spanned::Spanned};

fn hash_ident(ident: &Ident) -> TokenStream2 {
    let mut hash_field = TokenStream2::new();
    hash_field.append(Punct::new('#', Spacing::Joint));
    hash_field.append(ident.clone());
    hash_field
}

#[proc_macro_derive(Quine)]
pub fn derive_quine(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse2(input.into()).unwrap();
    let generics = input.generics;
    let ident = input.ident;
    let (impl_gen, ty_gen, where_clause) = generics.split_for_impl();
    let body = match input.data {
        // Derive for structs
        Data::Struct(data) => match &data.fields {
            Fields::Unit => {
                let path = hash_ident(&Ident::new(&"path", proc_macro2::Span::call_site()));
                quote! {
                    use syn::{parse_str, Path};
                    let fully_qualified_path = concat!("::",module_path!(), "::", stringify!(#ident));
                    let path: syn::Path = parse_str(fully_qualified_path).unwrap();
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
                    use syn::{parse_str, Path};
                    let fully_qualified_path = concat!("::",module_path!(), "::", stringify!(#ident));
                    let path: syn::Path = parse_str(fully_qualified_path).unwrap();
                    ::quote::quote!{#path(#(#exps),*)}
                }
            }
            Fields::Named(fields) => {
                let (decls, exps): (Vec<TokenStream2>, Vec<TokenStream2>) = fields
                    .named
                    .iter()
                    .map(|f| {
                        let idnt = f.ident.as_ref().unwrap();
                        let field_let = quote! {
                            let #idnt = self.#idnt.ctor_tokens();
                        };
                        let hash_idnt = hash_ident(idnt);
                        let field_exp = quote! {#idnt: #hash_idnt};
                        (field_let, field_exp)
                    })
                    .unzip();
                let path = hash_ident(&Ident::new(&"path", proc_macro2::Span::call_site()));
                quote! {
                    #(#decls)*
                    use syn::{parse_str, Path};
                    let fully_qualified_path = concat!("::",module_path!(), "::", stringify!(#ident));
                    let path: syn::Path = parse_str(fully_qualified_path).unwrap();
                    ::quote::quote!{#path{#(#exps),*}}
                }
            }
        },
        Data::Enum(data) => {
            let arms = data.variants.iter().map(|v| {
                let variant_idnt = &v.ident;
                match &v.fields {
                    Fields::Unit => {
                        let path = hash_ident(&Ident::new(&"path", proc_macro2::Span::call_site()));
                        quote! {#ident::#variant_idnt => {
                            use syn::{parse_str, Path};
                            let fully_qualified_path = concat!("::",module_path!(), "::", stringify!(#ident));
                            let path: syn::Path = parse_str(fully_qualified_path).unwrap();
                            ::quote::quote!{#path::#variant_idnt}
                        }}
                    }
                    Fields::Unnamed(fields) => {
                        let mut binds: Vec<Ident> = Vec::new();
                        let mut decls: Vec<TokenStream2> = Vec::new();
                        let mut exps: Vec<TokenStream2> = Vec::new();
                        for (i, f) in fields.unnamed.iter().enumerate() {
                            let idnt = Ident::new(format!("gen_field_{}", i).as_str(), f.span());
                            let exp_idnt =
                                Ident::new(format!("gen_field_{}_exp", i).as_str(), f.span());
                            let field_let = quote! {
                                let #exp_idnt = #idnt.ctor_tokens();
                            };
                            binds.push(idnt);
                            decls.push(field_let);
                            exps.push(hash_ident(&exp_idnt));
                        }
                        let path = hash_ident(&Ident::new(&"path", proc_macro2::Span::call_site()));
                        quote! {
                            #ident::#variant_idnt(#(#binds),*) => {
                                #(#decls)*
                                use syn::{parse_str, Path};
                                let fully_qualified_path = concat!("::",module_path!(), "::", stringify!(#ident));
                                let path: syn::Path = parse_str(fully_qualified_path).unwrap();
                                ::quote::quote!{#path::#variant_idnt(#(#exps),*)}
                            }
                        }
                    }
                    Fields::Named(fields) => {
                        let mut binds: Vec<Ident> = Vec::new();
                        let mut decls: Vec<TokenStream2> = Vec::new();
                        let mut exps: Vec<TokenStream2> = Vec::new();
                        for f in fields.named.iter() {
                            let idnt = f.ident.as_ref().unwrap();
                            let exp_idnt =
                                Ident::new(format!("gen_field_{}_exp", idnt).as_str(), f.span());
                            let field_let = quote! {
                                let #exp_idnt = #idnt.ctor_tokens();
                            };
                            let hash_idnt = hash_ident(&exp_idnt);

                            binds.push(idnt.clone());
                            decls.push(field_let);
                            exps.push(quote! { #idnt: #hash_idnt });
                        }
                        let path = hash_ident(&Ident::new(&"path", proc_macro2::Span::call_site()));
                        quote! {
                            #ident::#variant_idnt{#(#binds),*} => {
                                #(#decls)*
                                use syn::{parse_str, Path};
                                let fully_qualified_path = concat!("::",module_path!(), "::", stringify!(#ident));
                                let path: syn::Path = parse_str(fully_qualified_path).unwrap();
                                ::quote::quote!{#path::#variant_idnt{#(#exps),*}}
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
            fn ctor_tokens(&self) -> TokenStream {
                #body
            }
        }
    };
    ans.into()
}
