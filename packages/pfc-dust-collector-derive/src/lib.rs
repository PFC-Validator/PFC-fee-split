use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, DataEnum, DeriveInput};

/// Merges the variants of two enums.
///
/// Adapted from DAO DAO:
/// https://github.com/DA0-DA0/dao-contracts/blob/74bd3881fdd86829e5e8b132b9952dd64f2d0737/packages/dao-macros/src/lib.rs#L9
// Merges the variants of two enums.
fn merge_variants(metadata: TokenStream, left: TokenStream, right: TokenStream) -> TokenStream {
    use syn::Data::Enum;

    let args = parse_macro_input!(metadata as AttributeArgs);
    if let Some(first_arg) = args.first() {
        return syn::Error::new_spanned(first_arg, "macro takes no arguments")
            .to_compile_error()
            .into();
    }

    let mut left: DeriveInput = parse_macro_input!(left);
    let right: DeriveInput = parse_macro_input!(right);

    if let (
        Enum(DataEnum { variants, .. }),
        Enum(DataEnum {
            variants: to_add, ..
        }),
    ) = (&mut left.data, right.data)
    {
        variants.extend(to_add);

        quote! { #left }.into()
    } else {
        syn::Error::new(left.ident.span(), "variants may only be added for enums")
            .to_compile_error()
            .into()
    }
}

/// Append dust-collection execute messages variant(s) to an enum.
///
/// For example, apply the `pfc_dust_collect` macro to the following enum:
///
/// ```rust
/// use cosmwasm_schema::cw_serde;
/// use cw_ownable::cw_ownable_execute;
/// use pfc_dust_collector_derive::pfc_dust_collect;
///
/// #[pfc_dust_collect]
/// #[cw_serde]
/// enum ExecuteMsg {
///     Foo {},
///     Bar {},
/// }
/// ```
///
/// Is equivalent to:
///
/// ```rust
/// use cosmwasm_schema::cw_serde;
///
///
/// #[cw_serde]
/// enum ExecuteMsg {
///     DustReceived {},
///     FlushDust {},
///     SetReturnContract { contract:String },

/// }
/// ```
///
/// Note: `#[pfc_dust_collect]` must be applied _before_ `#[cw_serde]`.
#[proc_macro_attribute]
pub fn pfc_dust_collect(metadata: TokenStream, input: TokenStream) -> TokenStream {
    merge_variants(
        metadata,
        input,
        quote! {
            enum Right {
                ///  Receive dust from somewhere
                DustReceived {},
                /// Flush the dust
                FlushDust{} ,
                SetReturnContract{ contract: String },
            }
        }
        .into(),
    )
}
