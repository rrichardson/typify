// Copyright 2022 Oxide Computer Company

use std::collections::BTreeSet;

use proc_macro2::{Punct, Spacing, TokenStream, TokenTree};
use quote::{format_ident, quote, ToTokens};
use schemars::schema::Metadata;

use crate::{
    enums::{enum_impl, output_variant},
    structs::output_struct_property,
    util::{get_type_name, metadata_description},
    DefaultImpl, Name, TypeId, TypeSpace,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TypeEntryEnum {
    pub name: String,
    pub rename: Option<String>,
    pub description: Option<String>,
    pub default: Option<WrappedValue>,
    pub tag_type: EnumTagType,
    pub variants: Vec<Variant>,
    pub deny_unknown_fields: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TypeEntryStruct {
    pub name: String,
    pub rename: Option<String>,
    pub description: Option<String>,
    pub default: Option<WrappedValue>,
    pub properties: Vec<StructProperty>,
    pub deny_unknown_fields: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TypeEntryNewtype {
    pub name: String,
    pub rename: Option<String>,
    pub description: Option<String>,
    pub default: Option<WrappedValue>,
    pub type_id: TypeId,
    pub enum_values: Option<Vec<WrappedValue>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WrappedValue(pub serde_json::Value);
impl WrappedValue {
    pub(crate) fn new(value: serde_json::Value) -> Self {
        Self(value)
    }
}

impl Ord for WrappedValue {
    fn cmp(&self, _: &Self) -> std::cmp::Ordering {
        std::cmp::Ordering::Equal
    }
}
impl PartialOrd for WrappedValue {
    fn partial_cmp(&self, _: &Self) -> Option<std::cmp::Ordering> {
        Some(std::cmp::Ordering::Equal)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TypeEntry {
    pub details: TypeEntryDetails,
    pub derives: Option<BTreeSet<&'static str>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TypeEntryDetails {
    Enum(TypeEntryEnum),
    Struct(TypeEntryStruct),
    Newtype(TypeEntryNewtype),

    Option(TypeId),
    Box(TypeId),
    Array(TypeId),
    Map(TypeId),
    Set(TypeId),
    Tuple(Vec<TypeId>),
    Unit,
    /// Built-in complex types with no type generics such as Uuid
    BuiltIn(String),
    /// Boolean
    Boolean,
    /// Integers
    Integer(String),
    /// Floating point numbers; not Eq, Ord, or Hash
    Float(String),
    /// Strings... which we handle a little specially.
    String,

    /// While these types won't very make their way out to the user, we need
    /// reference types in particular to represent simple type aliases between
    /// types named as reference targets.
    Reference(TypeId),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum EnumTagType {
    External,
    Internal { tag: String },
    Adjacent { tag: String, content: String },
    Untagged,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Variant {
    pub name: String,
    pub rename: Option<String>,
    pub description: Option<String>,
    pub details: VariantDetails,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum VariantDetails {
    Simple,
    Tuple(Vec<TypeId>),
    Struct(Vec<StructProperty>),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StructProperty {
    pub name: String,
    pub rename: StructPropertyRename,
    pub state: StructPropertyState,
    pub description: Option<String>,
    pub type_id: TypeId,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum StructPropertyRename {
    None,
    Rename(String),
    Flatten,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum StructPropertyState {
    Required,
    Optional,
    Default(WrappedValue),
}

#[derive(Debug)]
pub(crate) enum DefaultKind {
    Intrinsic,
    Specific,
    Generic(DefaultImpl),
}

impl TypeEntryEnum {
    pub(crate) fn from_metadata(
        type_name: Name,
        metadata: &Option<Box<Metadata>>,
        tag_type: EnumTagType,
        variants: Vec<Variant>,
        deny_unknown_fields: bool,
    ) -> TypeEntryDetails {
        let name = get_type_name(&type_name, metadata).unwrap();
        let rename = None;
        let description = metadata_description(metadata);

        TypeEntryDetails::Enum(Self {
            name,
            rename,
            description,
            default: None,
            tag_type,
            variants,
            deny_unknown_fields,
        })
    }
}

impl TypeEntryStruct {
    pub(crate) fn from_metadata(
        type_name: Name,
        metadata: &Option<Box<Metadata>>,
        properties: Vec<StructProperty>,
        deny_unknown_fields: bool,
    ) -> TypeEntryDetails {
        let name = get_type_name(&type_name, metadata).unwrap();
        let rename = None;
        let description = metadata_description(metadata);
        let default = metadata
            .as_ref()
            .and_then(|m| m.default.as_ref())
            .cloned()
            .map(WrappedValue::new);

        TypeEntryDetails::Struct(Self {
            name,
            rename,
            description,
            default,
            properties,
            deny_unknown_fields,
        })
    }
}

impl TypeEntryNewtype {
    pub(crate) fn from_metadata(
        type_name: Name,
        metadata: &Option<Box<Metadata>>,
        type_id: TypeId,
    ) -> TypeEntryDetails {
        let name = get_type_name(&type_name, metadata).unwrap();
        let rename = None;
        let description = metadata_description(metadata);

        TypeEntryDetails::Newtype(Self {
            name,
            rename,
            description,
            default: None,
            type_id,
            enum_values: None,
        })
    }

    pub(crate) fn from_metadata_with_enum_values(
        type_name: Name,
        metadata: &Option<Box<Metadata>>,
        type_id: TypeId,
        enum_values: &[serde_json::Value],
    ) -> TypeEntryDetails {
        let name = get_type_name(&type_name, metadata).unwrap();
        let rename = None;
        let description = metadata_description(metadata);

        TypeEntryDetails::Newtype(Self {
            name,
            rename,
            description,
            default: None,
            type_id,
            enum_values: Some(enum_values.iter().cloned().map(WrappedValue::new).collect()),
        })
    }
}

impl From<TypeEntryDetails> for TypeEntry {
    fn from(details: TypeEntryDetails) -> Self {
        Self {
            details,
            derives: None,
        }
    }
}

impl TypeEntry {
    pub(crate) fn new_builtin<S: ToString>(type_name: S) -> Self {
        TypeEntry {
            details: TypeEntryDetails::BuiltIn(type_name.to_string()),
            derives: None,
        }
    }
    pub(crate) fn new_boolean() -> Self {
        TypeEntry {
            details: TypeEntryDetails::Boolean,
            derives: None,
        }
    }
    pub(crate) fn new_integer<S: ToString>(type_name: S) -> Self {
        TypeEntry {
            details: TypeEntryDetails::Integer(type_name.to_string()),
            derives: None,
        }
    }
    pub(crate) fn new_float<S: ToString>(type_name: S) -> Self {
        TypeEntry {
            details: TypeEntryDetails::Float(type_name.to_string()),
            derives: None,
        }
    }

    pub(crate) fn name(&self) -> Option<&String> {
        match &self.details {
            TypeEntryDetails::Enum(TypeEntryEnum { name, .. })
            | TypeEntryDetails::Struct(TypeEntryStruct { name, .. })
            | TypeEntryDetails::Newtype(TypeEntryNewtype { name, .. }) => Some(name),

            _ => None,
        }
    }

    pub(crate) fn output(&self, type_space: &TypeSpace) -> TokenStream {
        let mut derives = vec![
            quote! { Serialize },
            quote! { Deserialize },
            quote! { Debug },
            quote! { Clone },
        ];

        derives.extend(type_space.extra_derives.clone());

        match &self.details {
            TypeEntryDetails::Enum(TypeEntryEnum {
                name,
                rename,
                description,
                default,
                tag_type,
                variants,
                deny_unknown_fields,
            }) => {
                let doc = description.as_ref().map(|desc| quote! { #[doc = #desc] });

                // TODO this is a one-off for some useful traits
                if variants
                    .iter()
                    .all(|variant| matches!(variant.details, VariantDetails::Simple))
                {
                    derives.extend(
                        vec![
                            quote! { Copy },
                            quote! { PartialOrd },
                            quote! { Ord },
                            quote! { PartialEq },
                            quote! { Eq },
                            quote! { Hash },
                        ]
                        .into_iter(),
                    );
                }

                let mut serde_options = Vec::new();
                if let Some(old_name) = rename {
                    serde_options.push(quote! { rename = #old_name });
                }
                match tag_type {
                    EnumTagType::External => {}
                    EnumTagType::Internal { tag } => {
                        serde_options.push(quote! { tag = #tag });
                    }
                    EnumTagType::Adjacent { tag, content } => {
                        serde_options.push(quote! { tag = #tag });
                        serde_options.push(quote! { content = #content });
                    }
                    EnumTagType::Untagged => {
                        serde_options.push(quote! { untagged });
                    }
                }
                if *deny_unknown_fields {
                    serde_options.push(quote! { deny_unknown_fields });
                }
                let serde = if serde_options.is_empty() {
                    quote! {}
                } else {
                    quote! { #[serde( #( #serde_options ),* )] }
                };

                let type_name = format_ident!("{}", name);

                let variants_decl = variants
                    .iter()
                    .map(|variant| output_variant(variant, type_space, name))
                    .collect::<Vec<_>>();

                let enum_impl = enum_impl(&type_name, variants);

                let default_impl = default.as_ref().map(|value| {
                    let default_stream = self.output_value(type_space, &value.0).unwrap();
                    quote! {
                        impl Default for #type_name {
                            fn default() -> Self {
                                #default_stream
                            }
                        }
                    }
                });

                quote! {
                    #doc
                    #[derive(#(#derives),*)]
                    #serde
                    pub enum #type_name {
                        #(#variants_decl)*
                    }

                    #enum_impl
                    #default_impl
                }
            }

            TypeEntryDetails::Struct(TypeEntryStruct {
                name,
                rename,
                description,
                default,
                properties,
                deny_unknown_fields,
            }) => {
                let doc = description.as_ref().map(|desc| quote! { #[doc = #desc] });

                let mut serde_options = Vec::new();
                if let Some(old_name) = rename {
                    serde_options.push(quote! { rename = #old_name });
                }
                if *deny_unknown_fields {
                    serde_options.push(quote! { deny_unknown_fields });
                }
                let serde = if serde_options.is_empty() {
                    quote! {}
                } else {
                    quote! { #[serde( #( #serde_options ),* )] }
                };

                let type_name = format_ident!("{}", name);
                let (prop_streams, prop_defaults): (Vec<_>, Vec<_>) = properties
                    .iter()
                    .map(|prop| output_struct_property(prop, type_space, true, name))
                    .unzip();

                let default_impl = default.as_ref().map(|value| {
                    let default_stream = self.output_value(type_space, &value.0).unwrap();
                    quote! {
                        impl Default for #type_name {
                            fn default() -> Self {
                                #default_stream
                            }
                        }
                    }
                });

                quote! {
                    #doc
                    #[derive(#(#derives),*)]
                    #serde
                    pub struct #type_name {
                        #(#prop_streams)*
                    }

                    #(#prop_defaults)*

                    #default_impl
                }
            }

            TypeEntryDetails::Newtype(TypeEntryNewtype {
                name,
                rename,
                description,
                default,
                type_id,
                enum_values,
            }) => {
                let doc = description.as_ref().map(|desc| quote! { #[doc = #desc] });

                let serde = rename.as_ref().map(|old_name| {
                    quote! {
                        #[serde(rename = #old_name)]
                    }
                });

                let type_name = format_ident!("{}", name);
                let sub_type = type_space.id_to_entry.get(type_id).unwrap();
                let sub_type_name = sub_type.type_ident(type_space, false);

                let (vis, try_from_impl) = if let Some(enum_values) = enum_values {
                    let value_output = enum_values
                        .iter()
                        .map(|value| sub_type.output_value(type_space, &value.0));
                    let try_from_impl = quote! {
                        impl std::convert::TryFrom<#sub_type_name> for #type_name {
                            type Error = &'static str;

                            fn try_from(value: #sub_type_name) -> Result<Self, Self::Error> {
                                if ![
                                    #(#value_output,)*
                                ].contains(&value) {
                                    Err("invalid value")
                                } else {
                                    Ok(Self(value))
                                }
                            }
                        }
                    };
                    (quote! {}, try_from_impl)
                } else {
                    (quote! {pub}, quote! {})
                };

                let default_impl = default.as_ref().map(|value| {
                    let default_stream = self.output_value(type_space, &value.0).unwrap();
                    quote! {
                        impl Default for #type_name {
                            fn default() -> Self {
                                #default_stream
                            }
                        }
                    }
                });

                quote! {
                    #doc
                    #[derive(#(#derives),*)]
                    #serde
                    pub struct #type_name(#vis #sub_type_name);

                    impl std::ops::Deref for #type_name {
                        type Target = #sub_type_name;
                        fn deref(&self) -> &Self::Target {
                            &self.0
                        }
                    }

                    #default_impl
                    #try_from_impl
                }
            }

            // These types require no definition as they're already defined.
            TypeEntryDetails::BuiltIn(_)
            | TypeEntryDetails::Boolean
            | TypeEntryDetails::Integer(_)
            | TypeEntryDetails::Float(_)
            | TypeEntryDetails::String
            | TypeEntryDetails::Option(_)
            | TypeEntryDetails::Array(_)
            | TypeEntryDetails::Map(_)
            | TypeEntryDetails::Set(_)
            | TypeEntryDetails::Unit
            | TypeEntryDetails::Box(_)
            | TypeEntryDetails::Tuple(_) => quote! {},

            // We should never get here as reference types should only be used
            // in-flight, but never recorded into the type space.
            TypeEntryDetails::Reference(_) => unreachable!(),
        }
    }

    pub(crate) fn type_name(&self, type_space: &TypeSpace) -> String {
        self.type_ident(type_space, false).to_string()
    }

    pub(crate) fn type_ident(&self, type_space: &TypeSpace, external: bool) -> TokenStream {
        match &self.details {
            // Named types.
            TypeEntryDetails::Enum(TypeEntryEnum { name, .. })
            | TypeEntryDetails::Struct(TypeEntryStruct { name, .. })
            | TypeEntryDetails::Newtype(TypeEntryNewtype { name, .. }) => {
                match &type_space.type_mod {
                    Some(type_mod) if external => {
                        let type_mod = format_ident!("{}", type_mod);
                        let type_name = format_ident!("{}", name);
                        quote! { #type_mod :: #type_name }
                    }
                    _ => {
                        let type_name = format_ident!("{}", name);
                        quote! { #type_name }
                    }
                }
            }

            TypeEntryDetails::Option(id) => {
                let inner_ty = type_space
                    .id_to_entry
                    .get(id)
                    .expect("unresolved type id for option");
                let inner_ident = inner_ty.type_ident(type_space, external);

                // Flatten nested Option types. This would only happen if the
                // schema encoded it; it's an odd construction.
                match &inner_ty.details {
                    TypeEntryDetails::Option(_) => inner_ident,
                    _ => quote! { Option<#inner_ident> },
                }
            }

            TypeEntryDetails::Box(id) => {
                let inner_ty = type_space
                    .id_to_entry
                    .get(id)
                    .expect("unresolved type id for box");

                let item = inner_ty.type_ident(type_space, external);

                quote! { Box<#item> }
            }

            TypeEntryDetails::Array(id) => {
                let inner_ty = type_space
                    .id_to_entry
                    .get(id)
                    .expect("unresolved type id for array");
                let item = inner_ty.type_ident(type_space, external);

                quote! { Vec<#item> }
            }

            TypeEntryDetails::Map(type_id) => {
                let inner_ty = type_space
                    .id_to_entry
                    .get(type_id)
                    .expect("unresolved type id for map")
                    .type_ident(type_space, external);

                quote! { std::collections::HashMap<String, #inner_ty> }
            }

            TypeEntryDetails::Set(id) => {
                let inner_ty = type_space
                    .id_to_entry
                    .get(id)
                    .expect("unresolved type id for set");
                let item = inner_ty.type_ident(type_space, external);
                // TODO we'll want this to be a Set of some kind, but we need to get the derives right first.
                quote! { Vec<#item> }
            }

            TypeEntryDetails::Tuple(items) => {
                let type_streams = items.iter().map(|item| {
                    type_space
                        .id_to_entry
                        .get(item)
                        .expect("unresolved type id for tuple")
                        .type_ident(type_space, external)
                });

                quote! { ( #(#type_streams),* ) }
            }

            TypeEntryDetails::Unit => quote! { () },
            TypeEntryDetails::String => quote! { String },
            TypeEntryDetails::Boolean => quote! { bool },
            TypeEntryDetails::BuiltIn(name)
            | TypeEntryDetails::Integer(name)
            | TypeEntryDetails::Float(name) => syn::parse_str::<syn::TypePath>(name)
                .unwrap()
                .to_token_stream(),

            TypeEntryDetails::Reference(_) => panic!("references should be resolved by now"),
        }
    }

    pub(crate) fn type_parameter_ident(
        &self,
        type_space: &TypeSpace,
        lifetime_name: Option<&str>,
    ) -> TokenStream {
        let lifetime = lifetime_name.map(|s| {
            vec![
                TokenTree::from(Punct::new('\'', Spacing::Joint)),
                TokenTree::from(format_ident!("{}", s)),
            ]
            .into_iter()
            .collect::<TokenStream>()
        });
        match &self.details {
            // We special-case enums for which all variants are simple to let
            // them be passed as values rather than as references.
            TypeEntryDetails::Enum(TypeEntryEnum{ variants, .. })
                // TODO we should probably cache this rather than iterating
                // every time. We'll know it when the enum is constructed.
                if variants
                    .iter()
                    .all(|variant| matches!(variant.details, VariantDetails::Simple)) =>
            {
                self.type_ident(type_space, true)
            }

            TypeEntryDetails::Enum(_)
            | TypeEntryDetails::Struct(_)
            | TypeEntryDetails::Newtype(_)
            | TypeEntryDetails::Array(_)
            | TypeEntryDetails::Map(_)
            | TypeEntryDetails::Set(_)
            | TypeEntryDetails::Box(_)
            | TypeEntryDetails::BuiltIn(_) => {
                let ident = self.type_ident(type_space, true);
                quote! {
                    & #lifetime #ident
                }
            }

            TypeEntryDetails::Option(id) => {
                let inner_ty = type_space
                    .id_to_entry
                    .get(id)
                    .expect("unresolved type id for option");
                let inner_ident = inner_ty.type_parameter_ident(type_space, lifetime_name);

                // Flatten nested Option types. This would only happen if the
                // schema encoded it; it's an odd construction.
                match &inner_ty.details {
                    TypeEntryDetails::Option(_) => inner_ident,
                    _ => quote! { Option<#inner_ident> },
                }
            }
            TypeEntryDetails::Tuple(items) => {
                let type_streams = items.iter().map(|item| {
                    type_space
                        .id_to_entry
                        .get(item)
                        .expect("unresolved type id for tuple")
                        .type_parameter_ident(type_space, lifetime_name)
                });

                quote! { ( #(#type_streams),* ) }
            }

            TypeEntryDetails::Unit | TypeEntryDetails::Boolean|TypeEntryDetails::Integer(_) | TypeEntryDetails::Float(_) => {
                self.type_ident(type_space, true)
            }
            TypeEntryDetails::String => quote! { & #lifetime str },

            TypeEntryDetails::Reference(_) => panic!("references should be resolved by now"),
        }
    }

    pub(crate) fn describe(&self) -> String {
        match &self.details {
            TypeEntryDetails::Enum(TypeEntryEnum { name, .. }) => format!("enum {}", name),
            TypeEntryDetails::Struct(TypeEntryStruct { name, .. }) => format!("struct {}", name),
            TypeEntryDetails::Newtype(TypeEntryNewtype { name, type_id, .. }) => {
                format!("newtype {} {}", name, type_id.0)
            }

            TypeEntryDetails::Unit => "()".to_string(),
            TypeEntryDetails::Option(type_id) => format!("option {}", type_id.0),
            TypeEntryDetails::Array(type_id) => format!("array {}", type_id.0),
            TypeEntryDetails::Map(type_id) => format!("map {}", type_id.0),
            TypeEntryDetails::Set(type_id) => format!("set {}", type_id.0),
            TypeEntryDetails::Box(type_id) => format!("box {}", type_id.0),
            TypeEntryDetails::Tuple(type_ids) => {
                format!(
                    "tuple ({})",
                    type_ids
                        .iter()
                        .map(|type_id| type_id.0.to_string())
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
            TypeEntryDetails::Boolean => "bool".to_string(),
            TypeEntryDetails::BuiltIn(name)
            | TypeEntryDetails::Integer(name)
            | TypeEntryDetails::Float(name) => name.clone(),
            TypeEntryDetails::String => "string".to_string(),

            TypeEntryDetails::Reference(_) => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        type_entry::{TypeEntry, TypeEntryStruct},
        TypeEntryDetails, TypeSpace,
    };

    #[test]
    fn test_ident() {
        let ts = TypeSpace::default();

        let t = TypeEntry::new_integer("u32");
        let ident = t.type_ident(&ts, true);
        assert_eq!(ident.to_string(), "u32");
        let parameter = t.type_parameter_ident(&ts, None);
        assert_eq!(parameter.to_string(), "u32");

        let t = TypeEntry::from(TypeEntryDetails::String);
        let ident = t.type_ident(&ts, true);
        assert_eq!(ident.to_string(), "String");
        let parameter = t.type_parameter_ident(&ts, None);
        assert_eq!(parameter.to_string(), "& str");
        let parameter = t.type_parameter_ident(&ts, Some("static"));
        assert_eq!(parameter.to_string(), "& 'static str");

        let t = TypeEntry::from(TypeEntryDetails::Unit);
        let ident = t.type_ident(&ts, true);
        assert_eq!(ident.to_string(), "()");
        let parameter = t.type_parameter_ident(&ts, None);
        assert_eq!(parameter.to_string(), "()");

        let t = TypeEntry::from(TypeEntryDetails::Struct(TypeEntryStruct {
            name: "SomeType".to_string(),
            rename: None,
            description: None,
            default: None,
            properties: vec![],
            deny_unknown_fields: false,
        }));

        let ident = t.type_ident(&ts, true);
        assert_eq!(ident.to_string(), "SomeType");
        let parameter = t.type_parameter_ident(&ts, None);
        assert_eq!(parameter.to_string(), "& SomeType");
        let parameter = t.type_parameter_ident(&ts, Some("a"));
        assert_eq!(parameter.to_string(), "& 'a SomeType");
    }
}
