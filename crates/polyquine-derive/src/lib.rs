use proc_macro::TokenStream;
use proc_macro2::{Punct, Spacing, TokenStream as TokenStream2};
use quote::{TokenStreamExt, quote};
use syn::{Data, DeriveInput, Fields, Ident, Index, Path, spanned::Spanned};

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
                let path: ::syn::Path = ::syn::parse_quote!(::#prefix::#ident);
            }
        }
        None => {
            quote! {
                let fully_qualified_path = concat!("::", module_path!(), "::", stringify!(#ident));
                let path: ::syn::Path = ::syn::parse_str(fully_qualified_path).unwrap();
            }
        }
    }
}

#[proc_macro_derive(Quine, attributes(with_module))]
pub fn derive_quine(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse2(input.into()).unwrap();
    let generics = input.generics;
    let ident = input.ident;

    let mut module_prefix: Option<Path> = None;
    for attr in &input.attrs {
        if attr.path().is_ident("with_module") {
            if module_prefix.is_some() {
                return syn::Error::new(attr.span(), "duplicate with_module attribute")
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
    let body = match input.data {
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
                        let ident = f.ident.as_ref().unwrap();
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
            let arms = data.variants.iter().map(|v| {
                let variant_ident = &v.ident;
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
                            let ident = f.ident.as_ref().unwrap();
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
            fn ctor_tokens(&self) -> TokenStream {
                #body
            }
        }
    };
    ans.into()
}
