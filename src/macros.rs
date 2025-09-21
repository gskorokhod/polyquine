#[macro_export]
macro_rules! derive_primitive {
    ($type:ty) => {
        impl Quine for $type {
            fn ctor_tokens(&self) -> TokenStream {
                self.to_token_stream()
            }
        }
    };
    ($type:ty, $($rest:ty),*) => {
        derive_primitive!($type);
        derive_primitive!($($rest),*);
    };
}

/// Derive `Quine` for an iterable T where:
/// - T supports `.iter()` over its elements `&V`
/// - V implements `Quine`
/// - T implements `From<[V; N]>`
#[macro_export]
macro_rules! derive_iterable {
    // ::fully::qualified::GenericType<A, B, ...>
    ($(::)? $($pth:ident)::+ $type:ident < $($param:ident),* >) => {
        impl<$($param: Quine),*> Quine for ($($pth)::* $type)<$($param),*> {
            fn ctor_tokens(&self) -> TokenStream {
                let inner = self
                    .iter()
                    .map(|item| item.ctor_tokens())
                    .collect::<Vec<_>>();
                quote! {
                    $($pth)::* $type::from([#(#inner),*])
                }
            }
        }
    };

    // GenericType<A, B, ...>
    ($type:ident < $($param:ident),* >) => {
        impl<$($param: Quine),*> Quine for $type<$($param),*> {
            fn ctor_tokens(&self) -> TokenStream {
                let inner = self
                    .iter()
                    .map(|item| item.ctor_tokens())
                    .collect::<Vec<_>>();
                quote! {
                    $type::from([#(#inner),*])
                }
            }
        }
    };

    // NonGenericType
    ($type:ident) => {
        impl Quine for $type {
            fn ctor_tokens(&self) -> TokenStream {
                let inner = self
                    .iter()
                    .map(|item| item.ctor_tokens())
                    .collect::<Vec<_>>();
                quote! {
                    $type::from([#(#inner),*])
                }
            }
        }
    };
}

/// Derive the `Quine` trait for a tuple (T1, ..., Tn).
/// This is already done for some tuple sizes - use this macro if you encounter an unsupported tuple.
#[macro_export]
macro_rules! derive_tuple {
    ( $( $name:ident )+ ) => {
        impl<$($name: Quine),*> Quine for ($($name,)*)
        {
            fn ctor_tokens(&self) -> TokenStream {
                #[allow(non_snake_case)]
                let ($($name,)*) = self;
                let ctors = vec![$($name.ctor_tokens()),*];
                quote! {
                    (#(#ctors),*)
                }
            }
        }
    };
}

#[macro_export]
macro_rules! derive_tuple_all {
    ( $one:ident ) => {
        derive_tuple!($one);
    };

    ( $head:ident $($rest:ident)* ) => {
        derive_tuple!($head $($rest)*);
        derive_tuple_all!($($rest)*);
    };
}

/// Derive the `Quine` trait for an arbitrary type that implements `quote::ToTokens`.
/// The generated constructor is an expression in the form `T::from(#self)`.
#[macro_export]
macro_rules! derive_trivial {
    ($type:ty) => {
        impl Quine for $type {
            fn ctor_tokens(&self) -> TokenStream {
                quote! {
                    $type::from(#self)
                }
            }
        }
    };

    ($type:ty, $($rest:ty),*) => {
        derive_trivial!($type);
        derive_trivial!($($rest),*);
    }
}
