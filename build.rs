use protobuf::{
    descriptor::field_descriptor_proto::Type,
    reflect::{FieldDescriptor, MessageDescriptor},
};
use protobuf_codegen::{Customize, CustomizeCallback};

fn main() {
    println!("cargo:rerun-if-changed=src/protos");

    struct GenSerde;

    impl CustomizeCallback for GenSerde {
        fn message(&self, _message: &MessageDescriptor) -> Customize {
            Customize::default().before("#[derive(::serde::Serialize, ::serde::Deserialize)]\n#[serde(deny_unknown_fields)]")
        }

        fn oneof(&self, _oneof: &protobuf::reflect::OneofDescriptor) -> Customize {
            Customize::default()
                .before("#[derive(::serde::Serialize, ::serde::Deserialize, ::strum::EnumIter, ::strum::EnumString, ::strum::AsRefStr)]")
        }

        fn field(&self, field: &FieldDescriptor) -> Customize {
            if field.name() == "choices" && field.containing_message().name() == "Choice" {
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
            } else {
                Customize::default()
                    .before("#[serde(default, skip_serializing_if=\"crate::is_default_value\")]")
            }
        }

        fn special_field(&self, _message: &MessageDescriptor, _field: &str) -> Customize {
            Customize::default().before("#[serde(skip)]")
        }
    }

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
        .customize(Customize::default().lite_runtime(true))
        .customize_callback(GenSerde)
        .run_from_script();
}
