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
            Customize::default().before("#[derive(::serde::Serialize, ::serde::Deserialize)]")
        }

        fn oneof(&self, _oneof: &protobuf::reflect::OneofDescriptor) -> Customize {
            Customize::default()
                .before("#[derive(::serde::Serialize, ::serde::Deserialize, ::strum::EnumIter, ::strum::EnumString)]")
        }

        fn field(&self, field: &FieldDescriptor) -> Customize {
            if !field.is_repeated() && field.proto().type_() == Type::TYPE_MESSAGE {
                Customize::default().before(
                    "#[serde(serialize_with = \"crate::serialize_message\", deserialize_with = \"crate::deserialize_message\")]")
            } else {
                Customize::default()
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
        .customize_callback(GenSerde)
        .run_from_script();
}
