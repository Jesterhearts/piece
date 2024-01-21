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
                for (idx, field) in Card::descriptor().fields().enumerate() {
                    Self::render_field(
                        ui,
                        &mut self.dynamic_fields,
                        &mut self.dynamic_repeated_fields,
                        &mut self.dynamic_selections,
                        &mut self.card,
                        "card",
                        field,
                        idx,
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
        idx: usize,
        ui: &mut egui::Ui,
        message: &mut dyn MessageDyn,
        construct_value: impl FnOnce(T) -> ReflectValueBox,
    ) {
        let key = format!("{}_{}{}", prefix, field.full_name(), idx);
        let text = dynamic_fields.entry(key.clone()).or_default();
        let sense = ui.add(
            TextEdit::singleline(text)
                .id_source(key)
                .desired_width(100.0),
        );
        if sense.changed() || sense.lost_focus() {
            if let Ok(value) = text.parse::<T>() {
                field.set_singular_field(message, construct_value(value));
                info!("Set field in: {:?}", message);
            } else if text.is_empty() {
                field.clear_field(message);
                info!("Cleared field in: {:?}", message);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_oneof(
        ui: &mut egui::Ui,
        dynamic_fields: &mut HashMap<String, String>,
        dynamic_repeated_fields: &mut HashMap<String, Vec<String>>,
        dynamic_selections: &mut HashMap<String, usize>,
        message: &mut dyn MessageDyn,
        prefix: &str,
        message_descriptor: &protobuf::reflect::MessageDescriptor,
        oneof_name: &str,
        idx: usize,
    ) {
        ui.vertical(|ui| {
            for (field_idx, field) in message_descriptor
                .fields()
                .filter(|field| field.containing_oneof().is_none())
                .enumerate()
            {
                Self::render_field(
                    ui,
                    dynamic_fields,
                    dynamic_repeated_fields,
                    dynamic_selections,
                    message,
                    &format!("{}{}_oneof_field_{}", prefix, idx, field_idx),
                    field,
                    field_idx,
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

                for field in proto.fields() {
                    Self::render_field(
                        ui,
                        dynamic_fields,
                        dynamic_repeated_fields,
                        dynamic_selections,
                        message,
                        prefix,
                        field,
                        idx,
                    );
                }
            }
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn render_field(
        ui: &mut egui::Ui,
        dynamic_fields: &mut HashMap<String, String>,
        dynamic_repeated_fields: &mut HashMap<String, Vec<String>>,
        dynamic_selections: &mut HashMap<String, usize>,
        message: &mut dyn MessageDyn,
        prefix: &str,
        target: protobuf::reflect::FieldDescriptor,
        idx: usize,
    ) {
        ui.horizontal(|ui| {
            ui.label(target.name().to_case(Case::Title));

            if target.name() == "reduction" || target.name() == "mana_cost" {
                let key = format!("{}_{}{}", prefix, target, idx);
                let text = dynamic_fields.entry(key.clone()).or_default();

                let sense = ui.text_edit_singleline(text);

                if sense.changed() || sense.lost_focus() {
                    let text = format!(r#""{text}""#);
                    let deserializer = serde_yaml::Deserializer::from_str(&text);
                    match deserialize_mana_cost(deserializer) {
                        Ok(values) => {
                            let mut repeated = target.mut_repeated(message);
                            repeated.clear();
                            for value in values {
                                dbg!(repeated.element_type());
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
                                idx,
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
                                idx,
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
                                idx,
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
                                idx,
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
                                idx,
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
                                idx,
                                ui,
                                message,
                                ReflectValueBox::F64,
                            );
                        }
                        RuntimeType::Bool => {
                            Self::render_field_descriptor(
                                prefix,
                                dynamic_fields,
                                &target,
                                idx,
                                ui,
                                message,
                                ReflectValueBox::Bool,
                            );
                        }
                        RuntimeType::String => {
                            let key = format!("{}_{}{}", prefix, target, idx);
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
                            let key = format!("{}_{}{}", prefix, target.full_name(), idx);
                            let text = dynamic_fields.entry(key.clone()).or_default();

                            ui.horizontal(|ui| {
                                ui.label("value:");
                                let sense = ui.text_edit_singleline(text);
                                let changed = popup_all_options(
                                    ui,
                                    dynamic_selections,
                                    &key,
                                    idx,
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
                                        .collect_vec();

                                    ui.horizontal(|ui| {
                                        ui.label("type:");
                                        let key =
                                            format!("{}_{}{}", prefix, target.full_name(), idx);
                                        let text = dynamic_fields.entry(key.clone()).or_default();
                                        let sense = ui.text_edit_singleline(text);

                                        popup_all_options(
                                            ui,
                                            dynamic_selections,
                                            &key,
                                            idx,
                                            &sense,
                                            text,
                                            &inputs,
                                        );

                                        let oneof_name = text.to_case(Case::Snake);
                                        Self::render_oneof(
                                            ui,
                                            dynamic_fields,
                                            dynamic_repeated_fields,
                                            dynamic_selections,
                                            message,
                                            &format!(
                                                "{}_{}{}",
                                                prefix,
                                                descriptor.full_name(),
                                                idx,
                                            ),
                                            &descriptor,
                                            &oneof_name,
                                            idx,
                                        );
                                    });
                                } else {
                                    ui.vertical(|ui| {
                                        for (idx, sub_field) in descriptor.fields().enumerate() {
                                            Self::render_field(
                                                ui,
                                                dynamic_fields,
                                                dynamic_repeated_fields,
                                                dynamic_selections,
                                                message,
                                                &format!("{}_{}", prefix, target.full_name()),
                                                sub_field,
                                                idx,
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
                        RuntimeType::String => todo!(),
                        RuntimeType::VecU8 => todo!(),
                        RuntimeType::Enum(descriptor) => {
                            ui.vertical(|ui| {
                                let mut repeated = target.mut_repeated(message);

                                let inputs = descriptor
                                    .values()
                                    .map(|enum_| enum_.name().to_case(Case::Title))
                                    .collect_vec();
                                let key =
                                    format!("{}_repeated_{}{}", prefix, target.full_name(), idx);
                                let text = dynamic_repeated_fields.entry(key.clone()).or_default();

                                if repeated.is_empty() {
                                    text.clear();
                                }

                                for (idx, text) in text.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.label("value:");
                                        let sense = ui.text_edit_singleline(text);

                                        let changed = popup_all_options(
                                            ui,
                                            dynamic_selections,
                                            &key,
                                            idx,
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
                                        .collect_vec();

                                    let key = format!(
                                        "{}_repeated_{}{}",
                                        prefix,
                                        target.full_name(),
                                        idx
                                    );
                                    let mut text = dynamic_repeated_fields
                                        .entry(key.clone())
                                        .or_default()
                                        .clone();
                                    if repeated.is_empty() {
                                        text.clear();
                                    }

                                    for (idx, text) in text.iter_mut().enumerate() {
                                        ui.horizontal(|ui| {
                                            ui.label("type:");
                                            let sense = ui.text_edit_singleline(text);

                                            popup_all_options(
                                                ui,
                                                dynamic_selections,
                                                &key,
                                                idx,
                                                &sense,
                                                text,
                                                &inputs,
                                            );

                                            let mut value =
                                                repeated.get(idx).to_message().unwrap().clone_box();
                                            Self::render_oneof(
                                                ui,
                                                dynamic_fields,
                                                dynamic_repeated_fields,
                                                dynamic_selections,
                                                &mut *value,
                                                &key,
                                                &descriptor,
                                                &text.to_case(Case::Snake),
                                                idx,
                                            );

                                            repeated.set(idx, ReflectValueBox::Message(value));
                                        });
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
                                    let key = format!(
                                        "{}_repeated_{}{}",
                                        prefix,
                                        target.full_name(),
                                        idx
                                    );
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
                                        for field in descriptor.fields() {
                                            Self::render_field(
                                                ui,
                                                dynamic_fields,
                                                dynamic_repeated_fields,
                                                dynamic_selections,
                                                &mut *message,
                                                &key,
                                                field,
                                                idx,
                                            );
                                        }
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
                            let text = dynamic_repeated_fields
                                .entry(format!("{}_{}{}", prefix, target.full_name(), idx))
                                .or_default();

                            for text in text.iter_mut() {
                                ui.horizontal(|ui| {
                                    ui.label("type:");
                                    let sense = ui.text_edit_singleline(text);
                                    let changed = popup_all_options(
                                        ui,
                                        dynamic_selections,
                                        prefix,
                                        idx,
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
                                                ReflectValueBox::Message(Box::<Empty>::default()),
                                            );
                                        }
                                    }
                                });
                            }

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
                            let text = dynamic_repeated_fields
                                .entry(format!("{}_{}{}", prefix, target.full_name(), idx))
                                .or_default();

                            for text in text.iter_mut() {
                                ui.horizontal(|ui| {
                                    ui.label("subtype:");
                                    let sense = ui.text_edit_singleline(text);
                                    let changed = popup_all_options(
                                        ui,
                                        dynamic_selections,
                                        prefix,
                                        idx,
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
                                                ReflectValueBox::Message(Box::<Empty>::default()),
                                            );
                                        }
                                    }
                                });
                            }

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
                            let text = dynamic_repeated_fields
                                .entry(format!("{}_{}{}", prefix, target.full_name(), idx))
                                .or_default();

                            for text in text.iter_mut() {
                                ui.horizontal(|ui| {
                                    ui.label("keyword:");
                                    let sense = ui.text_edit_singleline(text);
                                    popup_all_options(
                                        ui,
                                        dynamic_selections,
                                        prefix,
                                        idx,
                                        &sense,
                                        text,
                                        &inputs,
                                    );
                                });
                            }

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
    idx: usize,
    sense: &egui::Response,
    text: &mut String,
    inputs: &[String],
) -> bool {
    static MATCHER: OnceLock<Mutex<Matcher>> = OnceLock::new();
    let matcher_lock = MATCHER.get_or_init(|| Mutex::new(Matcher::new(Config::DEFAULT)));
    let mut matcher = matcher_lock.lock().unwrap();

    let mut changed = false;

    let key = format!("{}{}", prefix, idx);
    let id = ui.make_persistent_id(&key);

    let matches = Pattern::new(
        text,
        CaseMatching::Ignore,
        Normalization::Smart,
        AtomKind::Fuzzy,
    )
    .match_list(inputs, &mut matcher);

    if sense.changed() {
        dynamic_selections.remove(&key);
    }

    if sense.has_focus() {
        let up_pressed =
            ui.input_mut(|input| input.consume_key(Modifiers::default(), Key::ArrowUp));
        let down_pressed =
            ui.input_mut(|input| input.consume_key(Modifiers::default(), Key::ArrowDown));

        if up_pressed {
            let selected = *dynamic_selections.entry(key.clone()).or_default();
            if selected == 0 {
                dynamic_selections.remove(&key);
            } else {
                *dynamic_selections.entry(key.clone()).or_default() = selected - 1;
            }
        } else if down_pressed {
            let selected = *dynamic_selections.entry(key.clone()).or_default();
            *dynamic_selections.entry(key.clone()).or_default() =
                usize::min(selected + 1, matches.len() - 1);
        }

        ui.memory_mut(|m| m.open_popup(id));
    }

    let enter_or_tab = ui.input_mut(|input| input.key_pressed(Key::Enter))
        || ui.input_mut(|input| input.key_pressed(Key::Tab));
    if enter_or_tab && ui.memory(|m| m.is_popup_open(id)) {
        changed = true;
        let selected = *dynamic_selections.entry(key.clone()).or_default();
        info!("Accepted {selected}");
        *text = matches[selected].0.clone();
    }

    egui::popup::popup_below_widget(ui, id, sense, |ui| {
        egui::ScrollArea::vertical().id_source(id).show(ui, |ui| {
            for (idx, (input, _)) in matches.iter().enumerate() {
                let mut selected = if let Some(o) = dynamic_selections.get(&key) {
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

    changed
}
