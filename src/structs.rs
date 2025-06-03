use proc_macro2::{Ident, TokenStream};
use syn::{Result, DataStruct};

pub fn expand_struct(ty_name: &Ident, data: &DataStruct) -> Result<TokenStream> {
    todo!("Handling structs is not yet implemented");
}
