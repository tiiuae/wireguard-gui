use std::collections::HashMap;
use std::path::PathBuf;

use relm4::{gtk::prelude::*, prelude::*};
use relm4_components::{alert::*, save_dialog::*};

use crate::{
    config::{write_configs_to_path, WireguardConfig},
    fields::*,
    generation_settings::*,
};

#[derive(Debug)]
pub struct GeneratorModel {
    fields: Controller<Fields>,
    visible: bool,
    save_dialog: Controller<SaveDialog>,
    // XXX: I haven't found simpler way to store state required to save generated configs.
    latest_generated_configs: Option<Vec<WireguardConfig>>,
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
    SaveGeneratedInPath(PathBuf),
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
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let fields_description = vec![
            ("Listen Port".into(), Some("51820".into())),
            ("Number of Clients".into(), Some("3".into())),
            ("CIDR".into(), Some("10.0.0.0/24".into())),
            ("Client Allowed IPs".into(), Some("0.0.0.0/0, ::/0".into())),
            ("Endpoint (Optional)".into(), Some("myserver.dyndns.org:51820".into())),
            // ("DNS (Optional)".into(), Some("DNS (Optional)".into())),
            ("Post-Up rule (Optional)".into(), Some("iptables -A FORWARD -i %i -j ACCEPT; iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE".into())),
            ("Post-Down rule (Optional)".into(), Some("iptables -D FORWARD -i %i -j ACCEPT; iptables -t nat -D POSTROUTING -o eth0 -j MASQUERADE".into()))
        ];
        let fields_settings = FieldsSettings { fields_description };
        let fields = Fields::builder().launch(fields_settings).forward(
            sender.input_sender(),
            |msg| match msg {
                FieldsOutput::FieldsMap(map) => Self::Input::Generate(map),
            },
        );

        let save_dialog = SaveDialog::builder()
            .transient_for_native(&root)
            .launch(SaveDialogSettings {
                accept_label: String::from("Export"),
                cancel_label: String::from("Cancel"),
                create_folders: true,
                is_modal: true,
                filters: vec![{
                    let filter = gtk::FileFilter::new();
                    filter.add_mime_type("application/x-tar");
                    // filter.add_pattern("*.tar");
                    filter
                }],
            })
            .forward(sender.input_sender(), |response| match response {
                SaveDialogResponse::Accept(path) => Self::Input::SaveGeneratedInPath(path),
                SaveDialogResponse::Cancel => Self::Input::Ignore,
            });

        let alert_dialog = Alert::builder()
            .transient_for(&root)
            .launch(AlertSettings {
                text: String::from("Error"),
                secondary_text: None,
                confirm_label: None,
                cancel_label: Some(String::from("Ok")),
                option_label: None,
                is_modal: true,
                destructive_accept: true,
            })
            .forward(sender.input_sender(), |_| Self::Input::Ignore);

        let model = Self {
            visible: false,
            fields,
            save_dialog,
            latest_generated_configs: None,
            alert_dialog,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            Self::Input::Show => self.visible = true,
            Self::Input::Hide => self.visible = false,
            Self::Input::AskForFieldsMap => {
                self.fields.emit(FieldsInput::Collect);
            }
            // FIXME: On the first run allows to save with all fields being empty.
            Self::Input::Generate(fields) => match GenerationSettings::try_from(fields) {
                Ok(settings) => {
                    self.latest_generated_configs = Some(settings.generate());
                    self.save_dialog
                        .emit(SaveDialogMsg::SaveAs("clients.tar".to_string()))
                }
                Err(e) => {
                    self.alert_dialog
                        .state()
                        .get_mut()
                        .model
                        .settings
                        .secondary_text = Some(e.into());
                    self.alert_dialog.emit(AlertMsg::Show);
                }
            },
            Self::Input::SaveGeneratedInPath(path) => {
                let cfgs = self.latest_generated_configs.take().unwrap();
                let (host_cfg, clients_cfgs) = cfgs.split_first().unwrap();
                write_configs_to_path(clients_cfgs.to_vec(), path).unwrap();
                sender
                    .output(Self::Output::GeneratedHostConfig(host_cfg.clone()))
                    .unwrap();
                sender.input(Self::Input::Hide);
            }
            Self::Input::Ignore => (),
        }
    }
}
