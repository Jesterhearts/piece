#[macro_use]
extern crate tracing;

mod ui;

use std::{
    collections::HashMap,
    env::current_dir,
    str::FromStr,
    sync::{Mutex, OnceLock},
};

use convert_case::{Case, Casing};
use egui::{Key, Modifiers, TextEdit};
use itertools::Itertools;
use native_dialog::FileDialog;
use nucleo_matcher::{
    pattern::{AtomKind, CaseMatching, Normalization, Pattern},
    Config, Matcher,
};
use piece_lib::{
    deserialize_mana_cost,
    protogen::{
        card::Card,
        comment,
        cost::ManaCost,
        empty::Empty,
        keywords::Keyword,
        types::{Subtype, Type},
    },
};
use protobuf::{
    reflect::{ReflectValueBox, RuntimeFieldType, RuntimeType},
    EnumFull, MessageDyn, MessageFull,
};

#[derive(Debug, Default)]
struct App {
    card: Card,

    dynamic_fields: HashMap<String, String>,
    dynamic_boolean_fields: HashMap<String, bool>,
    dynamic_repeated_fields: HashMap<String, Vec<String>>,
    dynamic_selections: HashMap<String, usize>,
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .pretty()
        .with_ansi(false)
        .with_line_number(true)
        .with_file(true)
        .with_target(false)
        .init();

    eframe::run_native(
        "Piece Editor",
        eframe::NativeOptions::default(),
        Box::new(move |_| Box::<App>::default()),
    )
    .unwrap();
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("Top").show(ctx, |ui| {
            if ui.button("Save").clicked() {
                let path = FileDialog::new()
                    .add_filter("YAML files", &["yaml"])
                    .set_filename(
                        &self
                            .card
                            .name
                            .to_case(Case::Snake)
                            .replace(['-', '\'', ',', '+', '"'], "_"),
                    )
                    .set_location(&current_dir().unwrap())
                    .show_save_single_file()
                    .unwrap();

                if let Some(path) = path {
                    std::fs::write(path, serde_yaml::to_string(&self.card).unwrap()).unwrap();
                }
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.expand_to_include_rect(ui.max_rect());
                for (idx, field) in Card::descriptor().fields().enumerate() {
                    if let Some(hidden) =
                        comment::exts::hidden.get(field.proto().options.get_or_default())
                    {
                        if hidden {
                            continue;
                        }
                    }

                    Self::render_field(
                        ui,
                        &mut self.dynamic_fields,
                        &mut self.dynamic_boolean_fields,
                        &mut self.dynamic_repeated_fields,
                        &mut self.dynamic_selections,
                        &mut self.card,
                        &format!("card_field{}", idx),
                        field,
                    );
                }
            });
        });
    }
}

impl App {
    fn render_field_descriptor<T: FromStr>(
        prefix: &str,
        dynamic_fields: &mut HashMap<String, String>,
        field: &protobuf::reflect::FieldDescriptor,
        ui: &mut egui::Ui,
        message: &mut dyn MessageDyn,
        construct_value: impl FnOnce(T) -> ReflectValueBox,
    ) {
        let key = format!("{}_{}", prefix, field.full_name());
        let text = dynamic_fields.entry(key.clone()).or_default();
        let sense = ui.add(TextEdit::singleline(text).desired_width(200.0));
        if sense.changed() || sense.lost_focus() {
            if let Ok(value) = text.parse::<T>() {
                field.set_singular_field(message, construct_value(value));
                info!("Set field {} in: {:?}", field.name(), message);
            } else if text.is_empty() {
                field.clear_field(message);
                info!("Cleared field {} in: {:?}", field.name(), message);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_oneof(
        ui: &mut egui::Ui,
        dynamic_fields: &mut HashMap<String, String>,
        dynamic_boolean_fields: &mut HashMap<String, bool>,
        dynamic_repeated_fields: &mut HashMap<String, Vec<String>>,
        dynamic_selections: &mut HashMap<String, usize>,
        message: &mut dyn MessageDyn,
        show_tooltip: bool,
        prefix: &str,
        message_descriptor: &protobuf::reflect::MessageDescriptor,
        oneof_name: &str,
    ) {
        ui.vertical(|ui| {
            for (idx, field) in message_descriptor
                .fields()
                .filter(|field| field.containing_oneof().is_none())
                .enumerate()
            {
                if let Some(hidden) =
                    comment::exts::hidden.get(field.proto().options.get_or_default())
                {
                    if hidden {
                        continue;
                    }
                }

                Self::render_field(
                    ui,
                    dynamic_fields,
                    dynamic_boolean_fields,
                    dynamic_repeated_fields,
                    dynamic_selections,
                    message,
                    &format!("{}_oneof_field_{}", prefix, idx),
                    field,
                );
            }

            if let Some(RuntimeType::Message(proto)) =
                message_descriptor.all_oneofs().find_map(|oneof| {
                    oneof.fields().find_map(|field| {
                        if field.name() == oneof_name {
                            Some(field.singular_runtime_type())
                        } else {
                            None
                        }
                    })
                })
            {
                let target = message_descriptor.field_by_name(oneof_name).unwrap();
                let message = target.mut_message(message);

                if let Some(options) = target.proto().options.as_ref() {
                    if let Some(comment) = comment::exts::comment.get(options) {
                        if show_tooltip {
                            egui::show_tooltip(ui.ctx(), egui::Id::new(&comment), |ui| {
                                ui.label(comment);
                            });
                        }
                    }
                }

                for (idx, field) in proto.fields().enumerate() {
                    if let Some(hidden) =
                        comment::exts::hidden.get(field.proto().options.get_or_default())
                    {
                        if hidden {
                            continue;
                        }
                    }

                    Self::render_field(
                        ui,
                        dynamic_fields,
                        dynamic_boolean_fields,
                        dynamic_repeated_fields,
                        dynamic_selections,
                        message,
                        &format!("{}_oneof_subfield_{}", prefix, idx),
                        field,
                    );
                }
            }
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn render_field(
        ui: &mut egui::Ui,
        dynamic_fields: &mut HashMap<String, String>,
        dynamic_boolean_fields: &mut HashMap<String, bool>,
        dynamic_repeated_fields: &mut HashMap<String, Vec<String>>,
        dynamic_selections: &mut HashMap<String, usize>,
        message: &mut dyn MessageDyn,
        prefix: &str,
        target: protobuf::reflect::FieldDescriptor,
    ) {
        ui.horizontal(|ui| {
            let sense = ui.label(target.name().to_case(Case::Title));

            if let Some(options) = target.proto().options.as_ref() {
                if let Some(comment) = comment::exts::comment.get(options) {
                    if sense.hovered() {
                        egui::show_tooltip(ui.ctx(), egui::Id::new(&comment), |ui| {
                            ui.label(comment);
                        });
                    }
                }
            }

            if target.name() == "reduction" || target.name() == "mana_cost" {
                let key = format!("{}_{}", prefix, target.full_name());
                let text = dynamic_fields.entry(key.clone()).or_default();

                let sense = ui.add(TextEdit::singleline(text).desired_width(200.0));

                if sense.changed() || sense.lost_focus() {
                    let text = format!(r#""{text}""#);
                    let deserializer = serde_yaml::Deserializer::from_str(&text);
                    match deserialize_mana_cost(deserializer) {
                        Ok(values) => {
                            let mut repeated = target.mut_repeated(message);
                            repeated.clear();
                            for value in values {
                                repeated.push(ReflectValueBox::Enum(
                                    ManaCost::enum_descriptor(),
                                    value.value(),
                                ));
                            }

                            info!("set mana cost to {:?}", repeated);
                        }
                        Err(e) => {
                            info!("Failed to parse mana cost {} - {}", text, e.to_string());
                        }
                    }
                }
            } else {
                match target.runtime_field_type() {
                    RuntimeFieldType::Singular(single) => match single {
                        RuntimeType::I32 => {
                            Self::render_field_descriptor(
                                prefix,
                                dynamic_fields,
                                &target,
                                ui,
                                message,
                                ReflectValueBox::I32,
                            );
                        }
                        RuntimeType::I64 => {
                            Self::render_field_descriptor(
                                prefix,
                                dynamic_fields,
                                &target,
                                ui,
                                message,
                                ReflectValueBox::I64,
                            );
                        }
                        RuntimeType::U32 => {
                            Self::render_field_descriptor(
                                prefix,
                                dynamic_fields,
                                &target,
                                ui,
                                message,
                                ReflectValueBox::U32,
                            );
                        }
                        RuntimeType::U64 => {
                            Self::render_field_descriptor(
                                prefix,
                                dynamic_fields,
                                &target,
                                ui,
                                message,
                                ReflectValueBox::U64,
                            );
                        }
                        RuntimeType::F32 => {
                            Self::render_field_descriptor(
                                prefix,
                                dynamic_fields,
                                &target,
                                ui,
                                message,
                                ReflectValueBox::F32,
                            );
                        }
                        RuntimeType::F64 => {
                            Self::render_field_descriptor(
                                prefix,
                                dynamic_fields,
                                &target,
                                ui,
                                message,
                                ReflectValueBox::F64,
                            );
                        }
                        RuntimeType::Bool => {
                            let key = format!("{}_{}", prefix, target.full_name());
                            let value = dynamic_boolean_fields.entry(key.clone()).or_default();
                            let sense = ui
                                .horizontal(|ui| {
                                    let sense = ui.radio_value(value, false, "false");
                                    sense.union(ui.radio_value(value, true, "true"))
                                })
                                .inner;

                            if sense.changed() || sense.clicked() {
                                target.set_singular_field(message, ReflectValueBox::Bool(*value));
                                info!("Set boolean field {} in: {:?}", target.name(), message);
                            }
                        }
                        RuntimeType::String => {
                            let key = format!("{}_{}", prefix, target.full_name());
                            let text = dynamic_fields.entry(key.clone()).or_default();
                            let sense = if target.name() == "oracle_text" {
                                ui.add(TextEdit::multiline(text).id_source(key))
                            } else {
                                ui.add(TextEdit::singleline(text).id_source(key))
                            };

                            if sense.changed() || sense.lost_focus() {
                                if text.is_empty() {
                                    target.clear_field(message);
                                    info!("Cleared field in: {:?}", message);
                                } else {
                                    target.set_singular_field(
                                        message,
                                        ReflectValueBox::String(text.to_string()),
                                    );
                                    info!("Set field in: {:?}", message);
                                }
                            }
                        }
                        RuntimeType::VecU8 => todo!(),
                        RuntimeType::Enum(descriptor) => {
                            let inputs = descriptor
                                .values()
                                .map(|enum_| enum_.name().to_case(Case::Title))
                                .collect_vec();
                            let key = format!("{}_{}", prefix, target.full_name());
                            let text = dynamic_fields.entry(key.clone()).or_default();

                            ui.horizontal(|ui| {
                                ui.label("value:");
                                let sense = ui.add(TextEdit::singleline(text).desired_width(200.0));
                                let (changed, _) = popup_all_options(
                                    ui,
                                    dynamic_selections,
                                    &key,
                                    &sense,
                                    text,
                                    &inputs,
                                );
                                if sense.lost_focus() || sense.changed() || changed {
                                    if let Some(value) = descriptor
                                        .value_by_name(&text.to_case(Case::ScreamingSnake))
                                    {
                                        info!("Set field to {}", value.name());
                                        target.set_singular_field(
                                            message,
                                            ReflectValueBox::Enum(descriptor, value.value()),
                                        );
                                    }
                                }
                            });
                        }
                        RuntimeType::Message(descriptor) => {
                            if target.has_field(message) {
                                let message = target.mut_message(message);
                                if descriptor.oneofs().any(|_| true) {
                                    let inputs = descriptor
                                        .all_oneofs()
                                        .flat_map(|oneof| {
                                            oneof
                                                .fields()
                                                .map(|field| field.name().to_case(Case::Title))
                                                .collect_vec()
                                        })
                                        .filter(|oneof| {
                                            let field = descriptor
                                                .field_by_name(&oneof.to_case(Case::Snake))
                                                .unwrap();
                                            let options = field.proto().options.get_or_default();

                                            if let Some(hidden) = comment::exts::hidden.get(options)
                                            {
                                                !hidden
                                            } else {
                                                true
                                            }
                                        })
                                        .collect_vec();

                                    ui.horizontal(|ui| {
                                        ui.label("type:");
                                        let key = format!("{}_{}", prefix, target.full_name());
                                        let text = dynamic_fields.entry(key.clone()).or_default();
                                        let sense =
                                            ui.add(TextEdit::singleline(text).desired_width(200.0));

                                        let (_, popup_text) = popup_all_options(
                                            ui,
                                            dynamic_selections,
                                            &key,
                                            &sense,
                                            text,
                                            &inputs,
                                        );

                                        let text = popup_text.unwrap_or_else(|| text.clone());
                                        let oneof_name = text.to_case(Case::Snake);
                                        Self::render_oneof(
                                            ui,
                                            dynamic_fields,
                                            dynamic_boolean_fields,
                                            dynamic_repeated_fields,
                                            dynamic_selections,
                                            message,
                                            sense.hovered() || sense.has_focus(),
                                            &format!("{}_{}", prefix, descriptor.full_name()),
                                            &descriptor,
                                            &oneof_name,
                                        );
                                    });
                                } else {
                                    ui.vertical(|ui| {
                                        for (idx, sub_field) in descriptor.fields().enumerate() {
                                            if let Some(hidden) = comment::exts::hidden
                                                .get(sub_field.proto().options.get_or_default())
                                            {
                                                if hidden {
                                                    continue;
                                                }
                                            }

                                            Self::render_field(
                                                ui,
                                                dynamic_fields,
                                                dynamic_boolean_fields,
                                                dynamic_repeated_fields,
                                                dynamic_selections,
                                                message,
                                                &format!(
                                                    "{}_{}_{}",
                                                    prefix,
                                                    target.full_name(),
                                                    idx
                                                ),
                                                sub_field,
                                            );
                                        }
                                    });
                                }
                            } else if ui.button("+").clicked() {
                                target.mut_message(message);
                            }

                            ui.separator();
                            if ui.button("reset").clicked() {
                                target.clear_field(message);
                            }
                        }
                    },
                    RuntimeFieldType::Repeated(repeated) => match repeated {
                        RuntimeType::I32 => todo!(),
                        RuntimeType::I64 => todo!(),
                        RuntimeType::U32 => todo!(),
                        RuntimeType::U64 => todo!(),
                        RuntimeType::F32 => todo!(),
                        RuntimeType::F64 => todo!(),
                        RuntimeType::Bool => todo!(),
                        RuntimeType::String => {
                            let mut repeated = target.mut_repeated(message);

                            ui.vertical(|ui| {
                                let key = format!("{}_repeated_{}", prefix, target.full_name());
                                let text = dynamic_repeated_fields.entry(key.clone()).or_default();

                                if repeated.is_empty() {
                                    text.clear();
                                }

                                for (idx, text) in text.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.label("value:");
                                        let sense =
                                            ui.add(TextEdit::singleline(text).desired_width(200.0));

                                        if sense.changed() || sense.lost_focus() {
                                            repeated
                                                .set(idx, ReflectValueBox::String(text.clone()));
                                        }
                                    });
                                }

                                ui.horizontal(|ui| {
                                    if ui.button("+").clicked() {
                                        text.push(Default::default());
                                        repeated.push(ReflectValueBox::String(String::default()));
                                    }
                                    if ui.button("-").clicked() {
                                        let mut copy = repeated
                                            .into_iter()
                                            .map(|v| v.to_str().unwrap().to_string())
                                            .collect_vec();

                                        text.pop();
                                        copy.pop();

                                        repeated.clear();
                                        for value in copy {
                                            repeated.push(ReflectValueBox::String(value));
                                        }
                                    }
                                    if ui.button("reset").clicked() {
                                        text.clear();
                                        repeated.clear();
                                    }
                                });
                            });
                        }
                        RuntimeType::VecU8 => todo!(),
                        RuntimeType::Enum(descriptor) => {
                            ui.vertical(|ui| {
                                let mut repeated = target.mut_repeated(message);

                                let inputs = descriptor
                                    .values()
                                    .map(|enum_| enum_.name().to_case(Case::Title))
                                    .collect_vec();
                                let key = format!("{}_repeated_{}", prefix, target.full_name());
                                let text = dynamic_repeated_fields.entry(key.clone()).or_default();

                                if repeated.is_empty() {
                                    text.clear();
                                }

                                for (idx, text) in text.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.label("value:");
                                        let sense =
                                            ui.add(TextEdit::singleline(text).desired_width(200.0));

                                        let (changed, _) = popup_all_options(
                                            ui,
                                            dynamic_selections,
                                            &format!("{}_{}", key, idx),
                                            &sense,
                                            text,
                                            &inputs,
                                        );

                                        if sense.changed() || sense.lost_focus() || changed {
                                            if let Some(value) = descriptor
                                                .value_by_name(&text.to_case(Case::ScreamingSnake))
                                            {
                                                info!("Set {} to {}", idx, value.name());
                                                repeated.set(
                                                    idx,
                                                    ReflectValueBox::Enum(
                                                        descriptor.clone(),
                                                        value.value(),
                                                    ),
                                                );
                                            }
                                        }
                                    });
                                }

                                ui.horizontal(|ui| {
                                    if ui.button("+").clicked() {
                                        text.push(Default::default());
                                        repeated
                                            .push(ReflectValueBox::Enum(descriptor.clone(), -1));
                                    }
                                    if ui.button("-").clicked() {
                                        let mut copy = repeated
                                            .into_iter()
                                            .map(|v| v.to_enum_value().unwrap())
                                            .collect_vec();

                                        text.pop();
                                        copy.pop();

                                        repeated.clear();
                                        for value in copy {
                                            repeated.push(ReflectValueBox::Enum(
                                                descriptor.clone(),
                                                value,
                                            ));
                                        }
                                    }
                                    if ui.button("reset").clicked() {
                                        text.clear();
                                        repeated.clear();
                                    }
                                });
                            });
                        }
                        RuntimeType::Message(descriptor) => {
                            let mut repeated = target.mut_repeated(message);

                            ui.vertical(|ui| {
                                if descriptor.oneofs().any(|_| true) {
                                    let inputs = descriptor
                                        .all_oneofs()
                                        .flat_map(|oneof| {
                                            oneof
                                                .fields()
                                                .map(|field| field.name().to_case(Case::Title))
                                                .collect_vec()
                                        })
                                        .filter(|oneof| {
                                            let field = descriptor
                                                .field_by_name(&oneof.to_case(Case::Snake))
                                                .unwrap();
                                            let options = field.proto().options.get_or_default();

                                            if let Some(hidden) = comment::exts::hidden.get(options)
                                            {
                                                !hidden
                                            } else {
                                                true
                                            }
                                        })
                                        .collect_vec();

                                    let key =
                                        format!("{}_repeated_{}", prefix, target.full_name(),);
                                    let mut text = dynamic_repeated_fields
                                        .entry(key.clone())
                                        .or_default()
                                        .clone();
                                    if repeated.is_empty() {
                                        text.clear();
                                    }

                                    for (idx, text) in text.iter_mut().enumerate() {
                                        ui.collapsing(
                                            format!("{}[{}]", target.name(), idx),
                                            |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label("type:");
                                                    let sense = ui.add(
                                                        TextEdit::singleline(text)
                                                            .desired_width(200.0),
                                                    );

                                                    let (_, popup_text) = popup_all_options(
                                                        ui,
                                                        dynamic_selections,
                                                        &format!("{}_{}", key, idx),
                                                        &sense,
                                                        text,
                                                        &inputs,
                                                    );

                                                    let text =
                                                        popup_text.unwrap_or_else(|| text.clone());
                                                    let mut value = repeated
                                                        .get(idx)
                                                        .to_message()
                                                        .unwrap()
                                                        .clone_box();

                                                    Self::render_oneof(
                                                        ui,
                                                        dynamic_fields,
                                                        dynamic_boolean_fields,
                                                        dynamic_repeated_fields,
                                                        dynamic_selections,
                                                        &mut *value,
                                                        sense.hovered() || sense.has_focus(),
                                                        &format!("{}_{}", key, idx),
                                                        &descriptor,
                                                        &text.to_case(Case::Snake),
                                                    );

                                                    repeated
                                                        .set(idx, ReflectValueBox::Message(value));
                                                });
                                            },
                                        );
                                    }

                                    ui.horizontal(|ui| {
                                        if ui.button("+").clicked() {
                                            text.push(Default::default());
                                            repeated.push(ReflectValueBox::Message(
                                                descriptor.new_instance(),
                                            ));
                                        }
                                        if ui.button("-").clicked() {
                                            let mut copy = repeated
                                                .into_iter()
                                                .map(|m| m.to_message().unwrap().clone_box())
                                                .collect_vec();
                                            text.pop();
                                            copy.pop();

                                            repeated.clear();
                                            for m in copy {
                                                repeated.push(ReflectValueBox::Message(m));
                                            }
                                        }
                                        if ui.button("reset").clicked() {
                                            text.clear();
                                            repeated.clear();
                                        }
                                    });

                                    *dynamic_repeated_fields.entry(key).or_default() = text;
                                } else {
                                    let key =
                                        format!("{}_repeated_{}", prefix, target.full_name(),);
                                    let mut text = dynamic_repeated_fields
                                        .entry(key.clone())
                                        .or_default()
                                        .clone();

                                    for (idx, mut message) in repeated
                                        .into_iter()
                                        .map(|m| m.to_message().unwrap().clone_box())
                                        .enumerate()
                                        .collect_vec()
                                    {
                                        ui.collapsing(
                                            format!("{}[{}]", target.name(), idx),
                                            |ui| {
                                                for (field_idx, field) in
                                                    descriptor.fields().enumerate()
                                                {
                                                    if let Some(hidden) = comment::exts::hidden
                                                        .get(field.proto().options.get_or_default())
                                                    {
                                                        if hidden {
                                                            continue;
                                                        }
                                                    }

                                                    Self::render_field(
                                                        ui,
                                                        dynamic_fields,
                                                        dynamic_boolean_fields,
                                                        dynamic_repeated_fields,
                                                        dynamic_selections,
                                                        &mut *message,
                                                        &format!("{}_{}_{}", key, idx, field_idx),
                                                        field,
                                                    );
                                                }
                                            },
                                        );

                                        repeated.set(idx, ReflectValueBox::Message(message));
                                    }

                                    ui.horizontal(|ui| {
                                        if ui.button("+").clicked() {
                                            text.push(Default::default());
                                            repeated.push(ReflectValueBox::Message(
                                                descriptor.new_instance(),
                                            ));
                                        }
                                        if ui.button("-").clicked() {
                                            let mut copy = repeated
                                                .into_iter()
                                                .map(|m| m.to_message().unwrap().clone_box())
                                                .collect_vec();
                                            text.pop();
                                            copy.pop();

                                            repeated.clear();
                                            for m in copy {
                                                repeated.push(ReflectValueBox::Message(m));
                                            }
                                        }
                                        if ui.button("reset").clicked() {
                                            text.clear();
                                            repeated.clear();
                                        }
                                    });

                                    *dynamic_repeated_fields.entry(key).or_default() = text;
                                }
                            });
                        }
                    },
                    RuntimeFieldType::Map(_, _) => {
                        let mut map = target.mut_map(message);
                        if target.name() == "types"
                            || target.name() == "add_types"
                            || target.name() == "remove_types"
                        {
                            let inputs = Type::enum_descriptor()
                                .values()
                                .map(|enum_| enum_.name().to_case(Case::Title))
                                .collect_vec();
                            let key = format!("{}_{}", prefix, target.full_name());
                            let text = dynamic_repeated_fields.entry(key.clone()).or_default();

                            ui.vertical(|ui| {
                                for (idx, text) in text.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.label("type:");
                                        let sense =
                                            ui.add(TextEdit::singleline(text).desired_width(200.0));
                                        let (changed, _) = popup_all_options(
                                            ui,
                                            dynamic_selections,
                                            &format!("{}_{}", key, idx),
                                            &sense,
                                            text,
                                            &inputs,
                                        );

                                        if sense.lost_focus() || sense.changed() || changed {
                                            if let Some(value) = Type::enum_descriptor()
                                                .value_by_name(&text.to_case(Case::ScreamingSnake))
                                            {
                                                info!("Set key to {}", value.name());

                                                map.insert(
                                                    ReflectValueBox::I32(value.value()),
                                                    ReflectValueBox::Message(
                                                        Box::<Empty>::default(),
                                                    ),
                                                );
                                            }
                                        }
                                    });
                                }
                            });

                            ui.horizontal(|ui| {
                                if ui.button("+").clicked() {
                                    text.push(Default::default());
                                }
                                if ui.button("reset").clicked() {
                                    text.clear();
                                    map.clear();
                                }
                            });
                        } else if target.name() == "subtypes"
                            || target.name() == "add_subtypes"
                            || target.name() == "remove_subtypes"
                        {
                            let inputs = Subtype::enum_descriptor()
                                .values()
                                .map(|enum_| enum_.name().to_case(Case::Title))
                                .collect_vec();
                            let key = format!("{}_{}", prefix, target.full_name());
                            let text = dynamic_repeated_fields.entry(key.clone()).or_default();

                            ui.vertical(|ui| {
                                for (idx, text) in text.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.label("subtype:");
                                        let sense =
                                            ui.add(TextEdit::singleline(text).desired_width(200.0));
                                        let (changed, _) = popup_all_options(
                                            ui,
                                            dynamic_selections,
                                            &format!("{}_{}", key, idx),
                                            &sense,
                                            text,
                                            &inputs,
                                        );
                                        if sense.lost_focus() || sense.changed() || changed {
                                            if let Some(value) = Subtype::enum_descriptor()
                                                .value_by_name(&text.to_case(Case::ScreamingSnake))
                                            {
                                                info!("Set key to {}", value.name());

                                                map.insert(
                                                    ReflectValueBox::I32(value.value()),
                                                    ReflectValueBox::Message(
                                                        Box::<Empty>::default(),
                                                    ),
                                                );
                                            }
                                        }
                                    });
                                }
                            });

                            ui.horizontal(|ui| {
                                if ui.button("+").clicked() {
                                    text.push(Default::default());
                                }
                                if ui.button("reset").clicked() {
                                    text.clear();
                                    map.clear();
                                }
                            });
                        } else if target.name() == "keywords"
                            || target.name() == "add_keywords"
                            || target.name() == "remove_keywords"
                        {
                            let inputs = Keyword::enum_descriptor()
                                .values()
                                .map(|enum_| enum_.name().to_case(Case::Title))
                                .collect_vec();
                            let key = format!("{}_{}", prefix, target.full_name());
                            let text = dynamic_repeated_fields.entry(key.clone()).or_default();

                            ui.vertical(|ui| {
                                for (idx, text) in text.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.label("keyword:");
                                        let sense =
                                            ui.add(TextEdit::singleline(text).desired_width(200.0));
                                        popup_all_options(
                                            ui,
                                            dynamic_selections,
                                            &format!("{}_{}", key, idx),
                                            &sense,
                                            text,
                                            &inputs,
                                        );
                                    });
                                }
                            });

                            ui.horizontal(|ui| {
                                if ui.button("+").clicked() {
                                    text.push(Default::default());
                                }
                                if ui.button("reset").clicked() {
                                    text.clear();
                                    map.clear();
                                }
                            });

                            let mut values = HashMap::<i32, u32>::default();
                            for text in text {
                                if let Some(value) = Keyword::enum_descriptor()
                                    .value_by_name(&text.to_case(Case::ScreamingSnake))
                                {
                                    *values.entry(value.value()).or_default() += 1
                                }
                            }

                            for (key, value) in values {
                                map.insert(ReflectValueBox::I32(key), ReflectValueBox::U32(value));
                            }
                        }
                    }
                }
            }
        });
    }
}

fn popup_all_options(
    ui: &mut egui::Ui,
    dynamic_selections: &mut HashMap<String, usize>,
    prefix: &str,
    sense: &egui::Response,
    text: &mut String,
    inputs: &[String],
) -> (bool, Option<String>) {
    static MATCHER: OnceLock<Mutex<Matcher>> = OnceLock::new();
    let matcher_lock = MATCHER.get_or_init(|| Mutex::new(Matcher::new(Config::DEFAULT)));
    let mut matcher = matcher_lock.lock().unwrap();

    let mut changed = false;

    let id = ui.make_persistent_id(prefix);

    let matches = Pattern::new(
        text,
        CaseMatching::Ignore,
        Normalization::Smart,
        AtomKind::Fuzzy,
    )
    .match_list(inputs, &mut matcher);

    if sense.changed() {
        dynamic_selections.remove(prefix);
    }

    if sense.has_focus() {
        let up_pressed =
            ui.input_mut(|input| input.consume_key(Modifiers::default(), Key::ArrowUp));
        let down_pressed =
            ui.input_mut(|input| input.consume_key(Modifiers::default(), Key::ArrowDown));

        if up_pressed {
            let selected = *dynamic_selections.entry(prefix.to_string()).or_default();
            if selected == 0 {
                dynamic_selections.remove(prefix);
            } else {
                *dynamic_selections.entry(prefix.to_string()).or_default() = selected - 1;
            }
        } else if down_pressed {
            if dynamic_selections.contains_key(prefix) {
                let selected = *dynamic_selections.entry(prefix.to_string()).or_default();
                *dynamic_selections.entry(prefix.to_string()).or_default() =
                    usize::min(selected + 1, matches.len() - 1);
            } else {
                dynamic_selections.insert(prefix.to_string(), 0);
            }
        }

        ui.memory_mut(|m| m.open_popup(id));
    }

    let enter_or_tab = ui.input_mut(|input| input.key_pressed(Key::Enter))
        || ui.input_mut(|input| input.key_pressed(Key::Tab));
    if enter_or_tab && ui.memory(|m| m.is_popup_open(id)) {
        changed = true;
        let selected = *dynamic_selections.entry(prefix.to_string()).or_default();
        info!("Accepted {selected}");
        *text = matches[selected].0.clone();
    }

    egui::popup::popup_below_widget(ui, id, sense, |ui| {
        egui::ScrollArea::vertical().id_source(id).show(ui, |ui| {
            for (idx, (input, _)) in matches.iter().enumerate() {
                let mut selected = if let Some(o) = dynamic_selections.get(prefix) {
                    *o == idx
                } else {
                    false
                };

                if ui.toggle_value(&mut selected, *input).clicked() {
                    changed = true;
                    *text = input.to_string();
                    ui.memory_mut(|m| m.close_popup());
                }
            }
        })
    });

    if sense.lost_focus() {
        ui.memory_mut(|m| m.close_popup());
    }

    (
        changed,
        dynamic_selections
            .get(prefix)
            .and_then(|selected| matches.get(*selected).map(|(s, _)| (*s).clone())),
    )
}
