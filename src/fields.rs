/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use std::collections::HashMap;

use relm4::factory::{FactoryComponent, FactoryHashMap, FactorySender};
use relm4::{gtk::prelude::*, prelude::*};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

#[derive(Debug)]
pub struct FieldSettings {
    initial_value: Option<String>,
}

#[derive(Debug)]
struct Field {
    name: String,
    value: Option<String>,
}

#[derive(Debug)]
pub enum FieldInput {
    /// Updates value of a field according to the new value in the input field.
    UpdateValue,
}

#[relm4::factory(pub)]
impl FactoryComponent for Field {
    type Init = FieldSettings;
    type Input = FieldInput;
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::Box;
    type Index = String;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_halign: gtk::Align::Center,
            set_spacing: 10,
            set_margin_all: 12,

            #[name(label)]
            gtk::Label {
                set_label: &self.name,
                set_width_chars: 3,
            },

            // TODO: Abstract to certain interface that will allow to use editable label and input field.
            #[name(input)]
            gtk::Entry {
                connect_changed => Self::Input::UpdateValue,
            }
        },
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        _sender: FactorySender<Self>,
    ) {
        match msg {
            Self::Input::UpdateValue => {
                let text = widgets.input.buffer().text().trim().to_string();
                if !text.is_empty() {
                    self.value = Some(text);
                }
            }
        }
    }

    fn init_model(settings: Self::Init, index: &Self::Index, _sender: FactorySender<Self>) -> Self {
        Self {
            name: index.clone(),
            value: settings.initial_value,
        }
    }
}

#[derive(Debug)]
pub struct FieldsSettings {
    pub fields_description: Vec<(String, Option<String>)>,
}

#[derive(Debug)]
pub struct Fields {
    fields: FactoryHashMap<String, Field>,
}

#[derive(Debug)]
pub enum FieldsInput {
    Collect,
}

#[derive(Debug)]
pub enum FieldsOutput {
    FieldsMap(HashMap<String, Option<String>>),
}

#[relm4::component(pub)]
impl SimpleComponent for Fields {
    type Init = FieldsSettings;
    type Input = FieldsInput;
    type Output = FieldsOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 5,
            set_margin_all: 5,

            append: model.fields.widget(),
        }
    }

    fn init(
        settings: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let fields_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        let mut fields = FactoryHashMap::builder().launch(fields_box).detach();

        for (name, initial_value) in settings.fields_description {
            fields.insert(name, FieldSettings { initial_value });
        }

        let model = Fields { fields };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            Self::Input::Collect => {
                let fields_map = self
                    .fields
                    .values()
                    .map(|f| (f.name.clone(), f.value.clone()))
                    .collect();
                sender.output(Self::Output::FieldsMap(fields_map)).unwrap();
            }
        }
    }
}
