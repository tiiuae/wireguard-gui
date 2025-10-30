/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use crate::{
    cli,
    config::{write_configs_to_path, WireguardConfig},
    fields::*,
    generation_settings::*,
};
use relm4::{gtk::prelude::*, prelude::*};
use relm4_components::alert::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct GeneratorModel {
    fields: Controller<Fields>,
    visible: bool,
    // XXX: I haven't found simpler way to store state required to save generated configs.
    latest_generated_configs: Option<WireguardConfig>,
    alert_dialog: Controller<Alert>,
}

#[derive(Debug)]
pub enum GeneratorInput {
    Show,
    #[doc(hidden)]
    Hide,
    #[doc(hidden)]
    AskForFieldsMap,
    #[doc(hidden)]
    Generate(HashMap<String, Option<String>>),
    #[doc(hidden)]
    Ignore,
}

#[derive(Debug)]
pub enum GeneratorOutput {
    GeneratedHostConfig(WireguardConfig),
}

#[relm4::component(pub)]
impl SimpleComponent for GeneratorModel {
    type Init = ();
    type Input = GeneratorInput;
    type Output = GeneratorOutput;

    view! {
        gtk::Window {
            set_title: Some("Generator"),
            #[watch]
            set_visible: model.visible,
            set_deletable: false,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                append: model.fields.widget(),

                gtk::Box {
                    gtk::Button {
                        set_label: "Cancel",
                        connect_clicked => Self::Input::Hide
                    },
                    gtk::Button {
                        set_label: "Generate",
                        connect_clicked => Self::Input::AskForFieldsMap,
                    }
                }
            }
        }
    }

    fn init(
        (): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let fields_description = vec![
            ("Tunnel interface name".into(), None),
            ("Tunnel interface ip".into(), None),
            ("Listen Port [default:51820]".into(), Some("51820".into())),
            ("Number of Peers [default:1]".into(), Some("1".into())),
        ];
        let fields_settings = FieldsSettings { fields_description };
        let fields = Fields::builder().launch(fields_settings).forward(
            sender.input_sender(),
            |msg| match msg {
                FieldsOutput::FieldsMap(map) => Self::Input::Generate(map),
            },
        );

        let alert_dialog = Alert::builder()
            .transient_for(&root)
            .launch(AlertSettings {
                text: Some(String::from("Error")),
                cancel_label: Some(String::from("Ok")),
                is_modal: true,
                destructive_accept: true,
                ..Default::default()
            })
            .forward(sender.input_sender(), |_| Self::Input::Ignore);

        let model = Self {
            visible: false,
            fields,
            latest_generated_configs: None,
            alert_dialog,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        let ui_error = |e: String| {
            self.alert_dialog
                .state()
                .get_mut()
                .model
                .settings
                .secondary_text = Some(e);
            self.alert_dialog.emit(AlertMsg::Show);
        };

        match msg {
            Self::Input::Show => self.visible = true,
            Self::Input::Hide => self.visible = false,
            Self::Input::AskForFieldsMap => {
                self.fields.emit(FieldsInput::Collect);
            }
            Self::Input::Generate(fields) => match GenerationSettings::try_from(fields) {
                Ok(settings) => {
                    let cfg = match settings.generate() {
                        Ok(cfg) => cfg,
                        Err(e) => {
                            ui_error(format!("Error generating config: {e}"));
                            return;
                        }
                    };

                    let Some(iface_name) = cfg.interface.name.clone() else {
                        ui_error("Interface name is missing in the generated config.".into());
                        return;
                    };

                    let cfg_path = cli::get_configs_dir().join(format!("{iface_name}.conf"));
                    if let Err(e) = write_configs_to_path(&[&cfg], &cfg_path) {
                        ui_error(format!("Error writing config to file: {e}"));
                    }

                    sender
                        .output(Self::Output::GeneratedHostConfig(cfg.clone()))
                        .unwrap();
                    sender.input(Self::Input::Hide);
                    self.latest_generated_configs = Some(cfg);
                }
                Err(e) => {
                    ui_error(e.into());
                }
            },
            Self::Input::Ignore => (),
        }
    }
}
