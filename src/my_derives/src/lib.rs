use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{DeriveInput, FieldsUnnamed};
use syn::{Error, Fields, Path, Variant};

#[proc_macro_derive(ZDisplay)]
pub fn z_display(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    match implement_z_display(&input) {
        Ok(output) => output.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn implement_z_display(input: &DeriveInput) -> syn::Result<TokenStream> {
    let enum_name = &input.ident;
    let variants = try_make_legal_input(input)?;
    let imp: TokenStream = {
        let match_arms = variants.iter().map(|(variant_ident, node)| match node {
            Node::NonLeaf { .. } => {
                quote! {
                    #enum_name::#variant_ident(inner) => &inner.to_string()
                }
            }
            Node::Leaf => quote! {
                _ => <&str>::from(self)
            },
        });

        quote! {
            impl ::std::fmt::Display for #enum_name {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    let displayed = match self {
                        #(#match_arms),*
                    };
                    write!(f, "{}", displayed)
                }
            }
        }
    };
    Ok(imp.into())
}

#[proc_macro_derive(MayStartWith)]
pub fn may_start_with(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    match implement_may_start_with(&input) {
        Ok(output) => output.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn implement_may_start_with(input: &DeriveInput) -> syn::Result<TokenStream> {
    let enum_name = &input.ident;
    let variants = try_make_legal_input(input)?;
    let imp: TokenStream = write_may_start_with(enum_name, &variants);

    Ok(imp.into())
}

fn write_may_start_with(enum_name: &Ident, variants: &[(Ident, Node)]) -> TokenStream {
    let if_blocks = variants.iter().map(|(variant_ident, node)| match node {
        Node::NonLeaf { child } => {
            quote! {
                if #child::may_start_with(value) {
                    return true;
                }
            }
        }
        Node::Leaf => quote! {
            if <&str>::from(#enum_name::#variant_ident).starts_with(value) {
                return true
            }
        },
    });

    quote! {
        impl #enum_name {
            fn may_start_with(value: impl AsRef<str>) -> bool {
                let value = value.as_ref();
                #(#if_blocks)*

                false

            }
        }
    }
}

#[proc_macro_derive(MyFromStrParse)]
pub fn my_from_str_parse(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    match impliment_my_from_str_parse(&input) {
        Ok(output) => output.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn impliment_my_from_str_parse(input: &DeriveInput) -> syn::Result<TokenStream> {
    let enum_name = &input.ident;
    let variants = try_make_legal_input(input)?;
    let from_str_impl: TokenStream = write_from_str(enum_name, &variants);

    Ok(quote! {
        #from_str_impl
    }
    .into())
}

#[proc_macro_derive(FromInnerType)]
pub fn from_inner_type(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    match implement_recursive(&input) {
        Ok(output) => output.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn implement_recursive(input: &DeriveInput) -> syn::Result<TokenStream> {
    let enum_name = &input.ident;

    let variants = try_make_legal_input(input)?;

    let into_string_impl: TokenStream = write_from_inner_type(enum_name, &variants);

    Ok(quote! {
        #into_string_impl
    }
    .into())
}

fn try_make_legal_input(input: &DeriveInput) -> Result<Vec<(Ident, Node)>, Error> {
    let variants = {
        let unchecked = match &input.data {
            syn::Data::Enum(data_enum) => &data_enum.variants,
            _ => {
                return Err(Error::new(
                    input.ident.span(),
                    "EnumRecursive can only be derived for enums",
                ))
            }
        };
        let mut variants = Vec::new();
        for v in unchecked {
            variants.push(custom_variant(v)?)
        }
        variants
    };
    Ok(variants)
}

fn write_from_inner_type(enum_name: &Ident, variants: &Vec<(Ident, Node)>) -> TokenStream {
    let arms = variants.iter().map(|(variant_ident, node)| match node {
        Node::NonLeaf { .. } => {
            quote! {
                #enum_name::#variant_ident(inner) => inner.into()
            }
        }
        Node::Leaf => quote! {
            leaf => leaf.into()
        },
    });

    quote! {
        impl<'a> From<#enum_name> for &'a str {
            fn from(value: #enum_name) -> &'a str {
                match value {
                    #(#arms),*
                }
            }
        }
    }
}

fn write_from_str(enum_name: &Ident, variants: &Vec<(Ident, Node)>) -> TokenStream {
    let if_blocks = variants.iter().map(|(variant_ident, node)| match node {
        Node::NonLeaf { child } => {
            quote! {
                if let Ok(inner_variant) = value.parse::<#child>() {
                    return Ok(#enum_name::#variant_ident(inner_variant));
                }
            }
        }
        Node::Leaf => quote! {
            let instance = #enum_name::#variant_ident;
            let required: &'static str = instance.into();

            if value == required {
                return Ok(#enum_name::#variant_ident);
            }
        },
    });

    quote! {
        impl ::std::str::FromStr for #enum_name {
            type Err = ();
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                #(#if_blocks)*
                Err(())
            }
        }
    }
}

enum Node {
    NonLeaf { child: Path },
    Leaf,
}

fn custom_variant(v: &Variant) -> syn::Result<(Ident, Node)> {
    let data = match &v.fields {
        Fields::Named(fields) => {
            return Err(Error::new(fields.span(), "expected single, unnamed field"))
        }
        Fields::Unnamed(FieldsUnnamed {
            unnamed: fields, ..
        }) => {
            if fields.len() == 1 {
                let single_field = fields.first().unwrap();
                let child = match &single_field.ty {
                    syn::Type::Path(tp) => &tp.path.clone(),
                    _ => return Err(Error::new(fields.span(), "field must the name of an enum!")),
                };
                Node::NonLeaf {
                    child: child.clone(),
                }
            } else {
                return Err(Error::new(
                    fields.span(),
                    "only allowed a single, unnamed field",
                ));
            }
        }
        Fields::Unit => Node::Leaf,
    };

    Ok((v.ident.clone(), data))
}
