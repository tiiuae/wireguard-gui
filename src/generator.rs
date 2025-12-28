/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use crate::{
    cli,
    config::{WireguardConfig, write_config_to_path},
    fields::*,
    generation_settings::*,
};
use log::trace;
use relm4::{gtk::prelude::*, prelude::*};
use relm4_components::alert::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct GeneratorModel {
    fields: Controller<Fields>,
    alert_dialog: Controller<Alert>,
    window: gtk::ApplicationWindow,
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
    Error(String),
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
        gtk::ApplicationWindow {
            set_title: Some("Generator"),
            set_deletable: false,
            set_hide_on_close: true,

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
            fields,
            alert_dialog,
            window: root.clone(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            Self::Input::Show => {
                self.window.present();
                trace!("Self::Input::Show");
                trace!("{:#?}", self.fields);
            }
            Self::Input::Hide => {
                self.window.hide();
                trace!("Self::Input::Hide");
                trace!("{:#?}", self.fields);
            }
            Self::Input::AskForFieldsMap => {
                self.fields.emit(FieldsInput::Collect);
            }
            Self::Input::Generate(fields) => match GenerationSettings::try_from(fields) {
                Ok(settings) => {
                    let cfg = match settings.generate() {
                        Ok(cfg) => cfg,
                        Err(e) => {
                            sender
                                .input(Self::Input::Error(format!("Error generating config: {e}")));
                            return;
                        }
                    };

                    let Some(iface_name) = cfg.interface.name.as_deref() else {
                        sender.input(Self::Input::Error(
                            "Interface name is missing in the generated config.".into(),
                        ));
                        return;
                    };

                    trace!("generated-cfg:{:#?}", cfg);

                    let cfg_path = cli::get_configs_dir().join(format!("{iface_name}.conf"));
                    if let Err(e) = write_config_to_path(&cfg, &cfg_path) {
                        sender.input(Self::Input::Error(format!(
                            "Error writing config to file: {e}"
                        )));
                        return;
                    }

                    if let Err(err) = sender.output(Self::Output::GeneratedHostConfig(cfg)) {
                        sender.input(Self::Input::Error(format!(
                            "Failed to send GeneratedHostConfig output: {err:?}"
                        )));
                        return;
                    }
                    sender.input(Self::Input::Hide);
                }
                Err(e) => {
                    sender.input(Self::Input::Error(e.into()));
                }
            },
            Self::Input::Error(msg) => {
                self.alert_dialog.emit(AlertMsg::Hide);

                self.alert_dialog
                    .state()
                    .get_mut()
                    .model
                    .settings
                    .secondary_text = Some(msg);
                self.alert_dialog.emit(AlertMsg::Show);
            }
            Self::Input::Ignore => (),
        }
    }
}
