use protobuf::{
    descriptor::field_descriptor_proto::Type,
    reflect::{FieldDescriptor, MessageDescriptor},
};
use protobuf_codegen::{Customize, CustomizeCallback};

fn main() {
    println!("cargo:rerun-if-changed=src/protos");

    struct GenSerde;

    impl CustomizeCallback for GenSerde {
        fn enumeration(&self, _enum_type: &protobuf::reflect::EnumDescriptor) -> Customize {
            Customize::default().before(
                r#"#[derive(
                    ::serde::Serialize,
                    ::serde::Deserialize,
                    ::strum::EnumIter,
                    ::strum::AsRefStr,
                    PartialOrd,
                    Ord,
                )]"#,
            )
        }

        fn message(&self, message: &MessageDescriptor) -> Customize {
            if message.fields().any(|field| {
                field.is_repeated_or_map()
                    || field.proto().has_oneof_index()
                    || field.proto().type_() == Type::TYPE_ENUM
                    || field.proto().type_() == Type::TYPE_MESSAGE
            }) {
                Customize::default().before("#[derive(::serde::Serialize, ::serde::Deserialize, Eq)]\n#[serde(deny_unknown_fields)]")
            } else {
                Customize::default().before("#[derive(::serde::Serialize, ::serde::Deserialize, Eq, Hash)]\n#[serde(deny_unknown_fields)]")
            }
        }

        fn oneof(&self, oneof: &protobuf::reflect::OneofDescriptor) -> Customize {
            if oneof.name() == "destination" {
                Customize::default().before(
                    r#"#[derive(
                    ::serde::Serialize,
                    ::serde::Deserialize,
                    ::strum::EnumIter,
                    ::strum::EnumString,
                    ::strum::AsRefStr,
                    ::derive_more::From,
                    Eq,
                    Hash,
                )]
                #[strum(ascii_case_insensitive)]"#,
                )
            } else {
                Customize::default().before(
                    r#"#[derive(
                    ::serde::Serialize,
                    ::serde::Deserialize,
                    ::strum::EnumIter,
                    ::strum::EnumString,
                    ::strum::AsRefStr,
                    ::derive_more::From,
                    Eq,
                )]
                #[strum(ascii_case_insensitive)]"#,
                )
            }
        }

        fn field(&self, field: &FieldDescriptor) -> Customize {
            if field.name() == "oracle_text" {
                Customize::default().before(
                    r#"#[serde(
                        default,
                        serialize_with="crate::serialize_oracle_text",
                        deserialize_with="crate::deserialize_oracle_text",
                    )]"#,
                )
            } else if field.name() == "counter" && field.proto().type_() == Type::TYPE_ENUM {
                Customize::default().before(
                    r#"#[serde(
                        default,
                        serialize_with="crate::serialize_counter",
                        deserialize_with="crate::deserialize_counter",
                    )]"#,
                )
            } else if field.name() == "typeline" {
                Customize::default().before(
                    r#"#[serde(
                        default,
                        serialize_with="crate::serialize_typeline",
                        deserialize_with="crate::deserialize_typeline",
                    )]"#,
                )
            } else if field.name() == "choices" && field.containing_message().name() == "Choice" {
                Customize::default().before(
                    r#"#[serde(
                        default,
                        serialize_with="crate::serialize_mana_choice",
                        deserialize_with="crate::deserialize_mana_choice",
                        skip_serializing_if="Vec::is_empty"
                    )]"#,
                )
            } else if field.name() == "gain" && field.containing_message().name() == "Specific" {
                Customize::default().before(
                    r#"#[serde(
                        default,
                        serialize_with="crate::serialize_gain_mana",
                        deserialize_with="crate::deserialize_gain_mana",
                        skip_serializing_if="Vec::is_empty"
                    )]"#,
                )
            } else if (field.name() == "types" && field.containing_message().name() != "Typeline")
                || field.name() == "add_types"
                || field.name() == "remove_types"
            {
                Customize::default().before(
                    r#"#[serde(
                        default,
                        serialize_with="crate::serialize_types",
                        deserialize_with="crate::deserialize_types",
                        skip_serializing_if="::std::collections::HashMap::is_empty"
                    )]"#,
                )
            } else if (field.name() == "subtypes"
                && field.containing_message().name() != "Typeline")
                || field.name() == "add_subtypes"
                || field.name() == "remove_subtypes"
            {
                Customize::default().before(
                    r#"#[serde(
                        default,
                        serialize_with="crate::serialize_subtypes",
                        deserialize_with="crate::deserialize_subtypes",
                        skip_serializing_if="::std::collections::HashMap::is_empty"
                    )]"#,
                )
            } else if field.name() == "keywords"
                || field.name() == "add_keywords"
                || field.name() == "remove_keywords"
            {
                Customize::default().before(
                    r#"#[serde(
                        default,
                        serialize_with="crate::serialize_keywords",
                        deserialize_with="crate::deserialize_keywords",
                        skip_serializing_if="::std::collections::HashMap::is_empty"
                    )]"#,
                )
            } else if field.name() == "reduction" || field.name() == "mana_cost" {
                Customize::default().before(
                    r#"#[serde(
                        default,
                        serialize_with="crate::serialize_mana_cost",
                        deserialize_with="crate::deserialize_mana_cost",
                        skip_serializing_if="Vec::is_empty"
                    )]"#,
                )
            } else if field.is_repeated() && field.proto().type_() == Type::TYPE_ENUM {
                Customize::default().before(
                    r#"#[serde(
                        default,
                        serialize_with="crate::serialize_enum_list",
                        deserialize_with="crate::deserialize_enum_list",
                        skip_serializing_if="Vec::is_empty"
                    )]"#,
                )
            } else if field.is_repeated() {
                Customize::default()
                    .before("#[serde(default, skip_serializing_if=\"Vec::is_empty\")]")
            } else if !field.is_repeated() && field.proto().type_() == Type::TYPE_MESSAGE {
                Customize::default().before(
                    r#"#[serde(
                        serialize_with = "crate::serialize_message",
                        deserialize_with = "crate::deserialize_message",
                        default,
                        skip_serializing_if="::protobuf::MessageField::is_none"
                    )]"#,
                )
            } else if !field.is_repeated() && field.proto().type_() == Type::TYPE_ENUM {
                Customize::default().before(
                    r#"#[serde(
                        serialize_with = "crate::serialize_enum",
                        deserialize_with = "crate::deserialize_enum",
                        default,
                    )]"#,
                )
            } else {
                Customize::default()
                    .before("#[serde(default, skip_serializing_if=\"crate::is_default_value\")]")
            }
        }

        fn special_field(&self, _message: &MessageDescriptor, _field: &str) -> Customize {
            Customize::default().before("#[serde(skip)]")
        }
    }

    std::fs::create_dir_all("src/protogen").expect("Failed to create protogen directory");

    protobuf_codegen::Codegen::new()
        .pure()
        // All inputs and imports from the inputs must reside in `includes` directories.
        .includes(["src/protos"])
        // Inputs must reside in some of include paths.
        .inputs(
            std::fs::read_dir("src/protos")
                .unwrap()
                .map(|f| f.unwrap().path()),
        )
        .out_dir("src/protogen")
        .customize_callback(GenSerde)
        .run_from_script();
}
