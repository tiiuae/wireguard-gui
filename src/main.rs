/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use gtk::prelude::*;
use log::{debug, error, info};
use nix::unistd::chown;
use relm4::factory::{DynamicIndex, FactoryVecDeque};
use relm4::prelude::*;
use relm4_components::alert::*;
use relm4_components::open_button::{OpenButton, OpenButtonSettings};
use relm4_components::open_dialog::OpenDialogSettings;
use relm4_components::save_dialog::*;
use std::os::unix::fs::MetadataExt;
use std::rc::Rc;
use std::{fs, path::PathBuf};
use syslog::{BasicLogger, Facility, Formatter3164};
use wireguard_gui::{cli::*, config::*, generator::*, overview::*, tunnel::*, utils::*};
struct App {
    tunnels: FactoryVecDeque<Tunnel>,
    selected_tunnel_idx: Option<usize>,
    overview: Controller<OverviewModel>,
    generator: Controller<GeneratorModel>,
    import_button: Controller<OpenButton>,
    alert_dialog: Controller<Alert>,
    export_dialog: Controller<SaveDialog>,
    init_err_buffer: Vec<String>,
    init_complete: bool,
}

#[derive(Debug)]
enum AppMsg {
    ShowOverview(usize),
    AddTunnel(Box<WireguardConfig>),
    RemoveTunnel(DynamicIndex),
    ImportTunnel(PathBuf),
    SaveConfigInitiate,
    SaveConfigFinish(Box<WireguardConfig>, Option<PathBuf>),
    AddPeer,
    ExportConfigInitiate,
    ExportConfigFinish(PathBuf),
    ShowGenerator,
    Error(String),
    Info(String),
    AddInitErrors(String),
    InitComplete,
    OverviewInitScripts(Vec<RoutingScripts>),
    OverviewInitIfaceBindings(Vec<String>),
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

                        append: model.import_button.widget(),

                        gtk::Button {
                            set_label: "Generate Configs",
                            connect_clicked => Self::Input::ShowGenerator,
                        }
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

                            gtk::Button {
                                set_label: "Export",
                                connect_clicked => Self::Input::ExportConfigInitiate,
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
        let (scripts, err) = extract_scripts_metadata();

        if err.is_some() {
            sender.input(Self::Input::AddInitErrors(err.unwrap()));
        }

        let binding_ifaces = get_binding_interfaces();

        let mut tunnels = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |output| match output {
                TunnelOutput::Remove(idx) => Self::Input::RemoveTunnel(idx),

                TunnelOutput::Error(msg) => Self::Input::Error(msg),
            });

        match wireguard_gui::utils::load_existing_configurations() {
            Ok((cfgs, err)) => {
                let mut g = tunnels.guard();
                /*
                if there is no match for saved
                BindingInterface and RoutingScriptName sections,
                remove them from the config files
                */

                for mut cfg in cfgs {
                    let mut needs_save = false;

                    // Validate binding interface
                    if let Some(ref iface) = cfg.interface.binding_iface {
                        if !binding_ifaces.contains(iface) {
                            cfg.interface.binding_iface = None;
                            needs_save = true;
                        }
                    }

                    // Validate routing script
                    if let Some(ref script_name) = cfg.interface.routing_script_name {
                        if !scripts.iter().any(|s| s.name == *script_name) {
                            // Clear all routing script related fields
                            cfg.interface.routing_script_name = None;
                            cfg.interface.pre_up = None;
                            cfg.interface.pre_down = None;
                            cfg.interface.post_up = None;
                            cfg.interface.post_down = None;
                            needs_save = true;
                        }
                    }
                    // Save the modified config back to disk if changes were made
                    if needs_save {
                        if let Some(ref name) = cfg.interface.name {
                            let path = get_configs_dir().join(format!("{name}.conf"));
                            if let Err(e) = write_configs_to_path(&[&cfg], &path) {
                                sender.input(Self::Input::AddInitErrors(format!(
                                    "Failed to update config '{}': {}",
                                    name, e
                                )));
                                continue;
                            }
                        }
                    }
                    g.push_back(cfg);
                }

                if err.is_some() {
                    sender.input(Self::Input::AddInitErrors(err.unwrap()));
                }
            }
            Err(err) => {
                let msg = format!("Could not load existing configurations: {:#?}", err);
                error!("{}", msg);
                sender.input(Self::Input::AddInitErrors(msg));
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
                        filter.set_name(Some("wireguard config files"));
                        filter.add_pattern("*.conf"); // Specific to .conf files
                        filter
                    }],
                },
                text: "Import Tunnel",
                recently_opened_files: None,
                max_recent_files: 0,
            })
            .forward(sender.input_sender(), Self::Input::ImportTunnel);

        let export_dialog = SaveDialog::builder()
            .launch(SaveDialogSettings {
                accept_label: String::from("Export"),
                cancel_label: String::from("Cancel"),
                create_folders: true,
                is_modal: true,
                filters: vec![{
                    let filter = gtk::FileFilter::new();
                    filter.set_name(Some("wireguard config files"));
                    filter.add_pattern("*.conf"); // Specific to .conf files
                    filter
                }],
            })
            .forward(sender.input_sender(), |response| match response {
                SaveDialogResponse::Accept(path) => Self::Input::ExportConfigFinish(path),
                SaveDialogResponse::Cancel => Self::Input::Ignore,
            });

        let overview = OverviewModel::builder()
            .launch(WireguardConfig::default())
            .forward(sender.input_sender(), |msg| match msg {
                OverviewOutput::SaveConfig(config, path) => {
                    Self::Input::SaveConfigFinish(config, path)
                }
                OverviewOutput::Error(msg) => Self::Input::Error(msg),
                OverviewOutput::AddInitErrors(msg) => Self::Input::AddInitErrors(msg),
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
            export_dialog,
            init_err_buffer: Vec::new(),
            init_complete: false,
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

        /* set routing scripts for overview model */
        sender
            .input_sender()
            .emit(AppMsg::OverviewInitScripts(scripts));

        /* set binding interfaces for overview model */
        sender
            .input_sender()
            .emit(AppMsg::OverviewInitIfaceBindings(binding_ifaces));

        gtk::glib::timeout_add_local_once(tokio::time::Duration::from_millis(100), move || {
            sender.input(AppMsg::InitComplete);
        });
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
                // 1) Lock and inspect the list
                let mut tunnels = self.tunnels.guard();
                if let Some(tunnel) = tunnels.get(idx.current_index()) {
                    let path = tunnel.path();

                    // 2) Attempt to delete the file
                    match fs::remove_file(&path) {
                        Ok(()) => {
                            log::info!("Deleted config file {}", path.display());
                            sender.input(Self::Input::Info(format!(
                                "Deleted config file {}",
                                path.display()
                            )));
                        }
                        Err(e) => {
                            // Other I/O errors (permission, in‑use, etc.)
                            log::error!("Failed to delete {}: {}", path.display(), e);
                            sender.input(Self::Input::Error(format!(
                                "Failed to delete {}: {}",
                                path.display(),
                                e
                            )));
                            return;
                        }
                    }
                }

                // 3) Now remove it from the in‑memory list
                tunnels.remove(idx.current_index());
            }
            Self::Input::ImportTunnel(path) => {
                let file_content = std::fs::read_to_string(&path);
                let res = file_content.as_ref().map(|c| parse_config(c));

                let Ok(Ok(mut config)) = res else {
                    sender.input(Self::Input::Error(format!("{res:#?}")));
                    return;
                };

                if config.interface.name.is_none() {
                    let name = path.file_stem().and_then(|s| s.to_str().map(str::to_owned));

                    match name {
                        Some(name) => config.interface.name = Some(name),
                        None => {
                            sender.input(Self::Input::Error(format!(
                                "Invalid file name: {}",
                                path.display()
                            )));
                            return;
                        }
                    }
                }

                let cfg_path = wireguard_gui::cli::get_configs_dir()
                    .join(format!("{}.conf", config.interface.name.as_ref().unwrap()));

                if let Err(e) = write_configs_to_path(&[&config], &cfg_path) {
                    sender.input(Self::Input::Error(format!(
                        "Error writing config to file: {e}"
                    )));
                    return;
                }

                sender.input(Self::Input::AddTunnel(Box::new(config)));
            }
            Self::Input::SaveConfigInitiate => {
                self.overview.emit(OverviewInput::CollectTunnel(None));
            }
            Self::Input::ExportConfigInitiate => self
                .export_dialog
                .emit(SaveDialogMsg::SaveAs("my_export.conf".into())),

            Self::Input::SaveConfigFinish(config, path) => {
                let is_path_none = path.is_none();
                let Some(idx) = self.selected_tunnel_idx else {
                    return;
                };
                if let Some(selected_tunnel) = self.tunnels.guard().get_mut(idx) {
                    if selected_tunnel.active {
                        sender.input(Self::Input::Error(
                            "Tunnel should be disabled before saving the configuration".into(),
                        ));
                        return;
                    }
                    let new_tunnel = Tunnel::new(*config);
                    let path = match path {
                        Some(p) if validate_export_path(&p) => p,
                        Some(_) => {
                            sender.input(Self::Input::Error(
                                "Config file can be exported only under 'home' directory".into(),
                            ));
                            return;
                        }
                        None => new_tunnel.path(),
                    };

                    if let Err(e) = fs::write(path.clone(), write_config(&new_tunnel.config)) {
                        sender.input(Self::Input::Error(format!("{e}")));
                        return;
                    }

                    // Change ownership of the file to the parent directory's UID and GID
                    let (uid, guid) = match path.parent().and_then(|p| fs::metadata(p).ok()) {
                        Some(metadata) => (metadata.uid(), metadata.gid()),
                        None => {
                            sender.input(Self::Input::Error(format!(
                                "Could not get metadata for path: {}",
                                path.display()
                            )));
                            return;
                        }
                    };

                    info!(
                        "Setting export config Path: {}, UID: {}, GID: {}",
                        path.display(),
                        uid,
                        guid
                    );

                    if let Err(e) = chown(&path, Some(uid.into()), Some(guid.into())) {
                        sender.input(Self::Input::Error(format!(
                            "Failed to change ownership of {}: {}",
                            path.display(),
                            e
                        )));
                        return;
                    }

                    sender.input(Self::Input::Info(format!(
                        "Configuration saved to {}",
                        path.display()
                    )));
                    if is_path_none {
                        selected_tunnel.update_from(new_tunnel);
                    }
                }
            }
            Self::Input::AddPeer => {
                self.overview.emit(OverviewInput::AddPeer);
            }
            Self::Input::ShowGenerator => {
                self.generator.emit(GeneratorInput::Show);
            }
            Self::Input::ExportConfigFinish(path) => {
                self.overview.emit(OverviewInput::CollectTunnel(Some(path)));
            }
            Self::Input::OverviewInitScripts(s) => {
                println!("main: {:#?}", s.clone());
                self.overview.emit(OverviewInput::InitRoutingScripts(s));
            }
            Self::Input::OverviewInitIfaceBindings(b) => {
                println!("main: {:#?}", b.clone());
                self.overview.emit(OverviewInput::InitIfaceBindings(b));
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
            Self::Input::AddInitErrors(msg) => {
                self.init_err_buffer.push(msg);
            }
            Self::Input::InitComplete => {
                self.init_complete = true;

                if !self.init_err_buffer.is_empty() {
                    // Number each error: 1) ..., 2) ..., 3) ...
                    let combined = self
                        .init_err_buffer
                        .iter()
                        .enumerate()
                        .map(|(i, err)| format!("{}) {}", i + 1, err))
                        .collect::<Vec<_>>()
                        .join("\n\n");

                    self.init_err_buffer.clear();
                    sender.input(Self::Input::Error(combined));
                }
            }
            Self::Input::Ignore => (),
        }
    }
}

fn main() {
    initialize_logger(get_log_output(), get_log_level_output());

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
    let empty: Vec<String> = vec![];
    let app = RelmApp::new("relm4.ghaf.wireguard-gui").with_args(empty);

    app.run::<App>(());
}

/// Initializes the logging system based on the selected feature and runtime configuration.
///
///   Configures either `stdout` logging or `syslog` based on user input.
///   Panics if an invalid log output is specified.
fn initialize_logger(log_output: LogOutput, log_level: log::Level) {
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
