/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use std::{fs, path::PathBuf};

use gtk::prelude::*;
use relm4::factory::{DynamicIndex, FactoryVecDeque};
use relm4::prelude::*;
use relm4_components::alert::*;
use relm4_components::open_button::{OpenButton, OpenButtonSettings};
use relm4_components::open_dialog::OpenDialogSettings;

use log::{debug, error};
use syslog::{BasicLogger, Facility, Formatter3164};
use wireguard_gui::{cli::*, config::*, generator::*, overview::*, tunnel::*};

struct App {
    tunnels: FactoryVecDeque<Tunnel>,
    selected_tunnel_idx: Option<usize>,
    overview: Controller<OverviewModel>,
    generator: Controller<GeneratorModel>,
    import_button: Controller<OpenButton>,
    alert_dialog: Controller<Alert>,
}

#[derive(Debug)]
enum AppMsg {
    ShowOverview(usize),
    AddTunnel(Box<WireguardConfig>),
    RemoveTunnel(DynamicIndex),
    ImportTunnel(PathBuf),
    SaveConfigInitiate,
    SaveConfigFinish(Box<WireguardConfig>),
    AddPeer,
    #[allow(dead_code)]
    ShowGenerator,
    Error(String),
    Info(String),
    Ignore,
}

#[relm4::component]
impl SimpleComponent for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();

    view! {
        gtk::Window {
            set_title: Some("Wireguard"),
            set_default_size: (480, 340),

            gtk::Paned {
                set_shrink_start_child: false,
                set_shrink_end_child: false,

                #[wrap(Some)]
                set_start_child = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::ScrolledWindow {
                        set_vexpand: true,
                        set_hexpand: true,
                        set_propagate_natural_width:true,
                        set_min_content_width: 200,
                        #[local_ref]
                        tunnels_list_box -> gtk::ListBox {}
                    },

                    gtk::Box {
                        // gtk::Button {
                        //     set_label: "Add Tunnel",
                        //     connect_clicked => Self::Input::AddTunnel(Box::default()),
                        // },

                        append: model.import_button.widget()

                        // gtk::Button {
                        //     set_label: "Generate Configs",
                        //     connect_clicked => Self::Input::ShowGenerator,
                        // }
                    },
                },
                #[wrap(Some)]
                set_end_child = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    #[name = "config_overview"]
                    gtk::Box {
                        set_vexpand: true,
                        set_hexpand: true,

                        // TODO: Just set property
                        match () {
                            () => model.overview.widget().clone(),
                        },
                    },

                    gtk::CenterBox {
                        #[wrap(Some)]
                        set_end_widget = &gtk::Box {
                            gtk::Button {
                                set_label: "Save",
                                connect_clicked => Self::Input::SaveConfigInitiate,
                            },

                            gtk::Button {
                                set_label: "Add Peer",
                                connect_clicked => Self::Input::AddPeer,
                            },
                        }
                    }
                },
            },
        }
    }

    fn init(
        _counter: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut tunnels = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |output| match output {
                TunnelOutput::Remove(idx) => Self::Input::RemoveTunnel(idx),

                TunnelOutput::Error(msg) => Self::Input::Error(msg),
            });

        match wireguard_gui::utils::load_existing_configurations() {
            Ok(cfgs) => {
                let mut g = tunnels.guard();

                for cfg in cfgs {
                    g.push_back(cfg);
                }
            }
            Err(err) => {
                error!("Could not load existing configurations: {:#?}", err);
            }
        };

        let import_button = OpenButton::builder()
            .launch(OpenButtonSettings {
                dialog_settings: OpenDialogSettings {
                    folder_mode: false,
                    accept_label: String::from("Import"),
                    cancel_label: String::from("Cancel"),
                    create_folders: false,
                    is_modal: true,
                    filters: vec![{
                        let filter = gtk::FileFilter::new();
                        filter.add_pattern("*.conf");
                        filter
                    }],
                },
                text: "Import Tunnel",
                recently_opened_files: None,
                max_recent_files: 0,
            })
            .forward(sender.input_sender(), Self::Input::ImportTunnel);

        let overview = OverviewModel::builder()
            .launch(WireguardConfig::default())
            .forward(sender.input_sender(), |msg| match msg {
                OverviewOutput::SaveConfig(config) => Self::Input::SaveConfigFinish(config),
            });

        let generator =
            GeneratorModel::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    GeneratorOutput::GeneratedHostConfig(cfg) => {
                        Self::Input::AddTunnel(Box::new(cfg))
                    }
                });

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

        let model = App {
            tunnels,
            selected_tunnel_idx: None,
            import_button,
            overview,
            generator,
            alert_dialog,
        };

        let tunnels_list_box = model.tunnels.widget();

        tunnels_list_box.connect_row_selected(gtk::glib::clone!(
            #[strong]
            sender,
            move |_, row| {
                if let Some(lbr) = row {
                    sender
                        .input_sender()
                        .emit(AppMsg::ShowOverview(lbr.index().try_into().unwrap()));
                }
            }
        ));

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            Self::Input::ShowOverview(idx) => {
                self.selected_tunnel_idx = Some(idx);
                let tunnel = self.tunnels.get(idx).unwrap();
                self.overview
                    .emit(OverviewInput::ShowConfig(Box::new(tunnel.config.clone())));
            }
            Self::Input::AddTunnel(config) => {
                let mut tunnels = self.tunnels.guard();

                if tunnels
                    .iter()
                    .any(|t| t.config.interface.name == config.interface.name)
                {
                    sender.input(Self::Input::Error(format!(
                        "Tunnel with name {} already exists",
                        config.interface.name.as_ref().unwrap()
                    )));
                    return;
                }

                tunnels.push_back(*config);
            }
            Self::Input::RemoveTunnel(idx) => {
                let mut tunnels = self.tunnels.guard();
                // self.tunnels.widget.selection
                tunnels.remove(idx.current_index());
            }
            Self::Input::ImportTunnel(path) => {
                let file_content = std::fs::read_to_string(&path);
                let res = file_content.map(|c| parse_config(&c));

                let Ok(Ok(mut config)) = res else {
                    sender.input(Self::Input::Error(format!("{:#?}", res)));
                    return;
                };

                if config.interface.name.is_none() {
                    config.interface.name = path
                        .file_stem()
                        .unwrap_or_else(|| todo!())
                        .to_str()
                        .map(|s| s.to_owned());
                }

                sender.input(Self::Input::AddTunnel(Box::new(config)));
            }
            Self::Input::SaveConfigInitiate => self.overview.emit(OverviewInput::CollectTunnel),
            Self::Input::SaveConfigFinish(config) => {
                let Some(idx) = self.selected_tunnel_idx else {
                    return;
                };
                if let Some(selected_tunnel) = self.tunnels.guard().get_mut(idx) {
                    if selected_tunnel.active {
                        sender.input(Self::Input::Error(
                            "Tunnel should be disabled before saving the configuration".to_string(),
                        ));
                        return;
                    }
                    let new_tunnel = Tunnel::new(*config);
                    if let Err(e) = fs::write(new_tunnel.path(), write_config(&new_tunnel.config)) {
                        sender.input(Self::Input::Error(format!("{:#?}", e)));
                        return;
                    }
                    sender.input(Self::Input::Info(format!(
                        "Configuration saved to {:#?}",
                        new_tunnel.path()
                    )));

                    *selected_tunnel = new_tunnel.clone();
                }
            }
            Self::Input::AddPeer => {
                self.overview.emit(OverviewInput::AddPeer);
            }
            Self::Input::ShowGenerator => {
                self.generator.emit(GeneratorInput::Show);
            }
            Self::Input::Error(msg) => {
                self.alert_dialog
                    .state()
                    .get_mut()
                    .model
                    .settings
                    .secondary_text = Some(msg);
                self.alert_dialog.state().get_mut().model.settings.text =
                    Some(String::from("Error"));

                self.alert_dialog.emit(AlertMsg::Show);
            }
            Self::Input::Info(msg) => {
                self.alert_dialog
                    .state()
                    .get_mut()
                    .model
                    .settings
                    .secondary_text = Some(msg);
                self.alert_dialog.state().get_mut().model.settings.text =
                    Some(String::from("Info"));
                self.alert_dialog.emit(AlertMsg::Show);
            }
            Self::Input::Ignore => (),
        }
    }
}

fn main() {
    initialize_logger(get_log_output(), get_log_level());

    karen::builder()
        .wrapper("pkexec")
        .with_env(&[
            "DISPLAY",
            "XAUTHORITY",
            "WAYLAND_DISPLAY",
            "XDG_RUNTIME_DIR",
            "XDG_DATA_DIRS",
            "LIBGL_ALWAYS_SOFTWARE",
            "PATH",
        ])
        .unwrap();

    let app = RelmApp::new("relm4.ghaf.wireguard-gui");
    app.run::<App>(());
}

/// Initializes the logging system based on the selected feature and runtime configuration.
///
///   Configures either `stdout` logging or `syslog` based on user input.
///   Panics if an invalid log output is specified.
fn initialize_logger(log_output: &LogOutput, log_level: &log::Level) {
    let log_level = log_level.to_level_filter();

    match log_output {
        LogOutput::Stdout => {
            println!("Redirecting logger to stdout");
            env_logger::Builder::new().filter_level(log_level).init();
        }
        LogOutput::Syslog => {
            println!("Redirecting logger to syslog");
            let formatter = Formatter3164 {
                facility: Facility::LOG_USER,
                hostname: None,
                process: "wireguard-gui".into(),
                pid: 0,
            };

            let logger = match syslog::unix(formatter) {
                Err(e) => {
                    panic!("failed to connect to syslog: {e}");
                }
                Ok(logger) => logger,
            };

            log::set_boxed_logger(Box::new(BasicLogger::new(logger)))
                .expect("Failed to set logger");
            log::set_max_level(log_level);
        }
    }

    debug!("Logger initialized");
}
