/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use crate::gtk::gdk_pixbuf;
use anyhow::Context;
use gtk::prelude::*;
use log::{debug, error, info, trace};
use relm4::abstractions::Toaster;
use relm4::factory::{DynamicIndex, FactoryVecDeque};
use relm4::prelude::*;
use relm4_components::alert::*;
use relm4_components::open_button::{OpenButton, OpenButtonSettings};
use relm4_components::open_dialog::OpenDialogSettings;
use relm4_components::save_dialog::*;
use crate::gtk::pango;
use std::{fs, path::PathBuf};
use syslog::{BasicLogger, Facility, Formatter3164};
use wireguard_gui::{cli::*, config::*, generator::*, overview::*, tunnel::*, utils::*};

const GHAF_LOGO: &[u8] = include_bytes!("../assets/ghaf-logo.png");
const WG_LOGO: &[u8] = include_bytes!("../assets/wireguard-logo.png");
struct App {
    tunnels: FactoryVecDeque<Tunnel>,
    selected_tunnel_idx: Option<usize>,
    overview: Controller<OverviewModel>,
    generator: Controller<GeneratorModel>,
    import_button: Controller<OpenButton>,
    alert_dialog: Controller<Alert>,
    export_dialog: Controller<SaveDialog>,
    toaster: Toaster,
    init_err_buffer: Vec<String>,
    init_complete: bool,
    save_button_enabled: bool,
}

#[derive(Debug)]
enum AppMsg {
    ShowOverview(usize),
    AddTunnel {
        config: Box<WireguardConfig>,
        set_default: bool,
    },
    RemoveTunnel(DynamicIndex),
    ImportTunnel(PathBuf),
    ProcessImportedTunnel(Box<WireguardConfig>, PathBuf),
    SaveConfigInitiate,
    SaveConfigFinish(Box<WireguardConfig>, Option<PathBuf>),
    UpdateTunnel {
        idx: usize,
        new_tunnel_data: Box<TunnelData>,
        is_save_clicked: bool,
    },
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
    TunnelModified,
    OpenUrl(String),
    InitSyncFinished {
        scripts: Vec<RoutingScripts>,
        binding_ifaces: Vec<String>,
        loaded_configs: Vec<WireguardConfig>,
        initial_errors: Vec<String>,
    },
    Ignore,
}

#[relm4::component]
impl SimpleComponent for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();

    view! {
        adw::Window {
            set_title: Some("Wireguard GUI"),
            set_default_size: (480, 340),

            #[local_ref]
            toast_overlay -> adw::ToastOverlay {
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,

                    adw::HeaderBar {},

                    gtk::Paned {
                        set_shrink_start_child: false,
                        set_shrink_end_child: false,
                        set_vexpand: true,

                        #[wrap(Some)]
                        set_start_child = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_hexpand: true,
                            set_vexpand: true,
                            set_spacing: 10,
                            set_margin_start: 0,
                            set_margin_end: 0,
                            set_margin_top: 0,
                            set_margin_bottom: 0,

                    // Horizontal box to hold the two logos side by side
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_hexpand: false,
                        set_vexpand: false,
                        set_spacing: 5,
                        set_margin_start: 0,
                        set_margin_end: 0,
                        set_margin_top: 0,
                        set_margin_bottom: 0,


                    // Logo 1
                    gtk::Image {
                       set_from_pixbuf: ghaf_pixbuf.ok().as_ref(),
                       set_halign: gtk::Align::Fill,
                       set_valign: gtk::Align::Fill,
                       set_pixel_size: 100,
                       set_hexpand: true,
                       set_vexpand: true,
                    },

                    // Logo 2
                    gtk::Image {
                        set_from_pixbuf: wg_pixbuf.ok().as_ref(),
                        set_halign: gtk::Align::Fill,
                        set_valign: gtk::Align::Fill,
                        set_pixel_size: 150,
                        set_hexpand: true,
                        set_vexpand: true,
                     },

                    },

                    gtk::ScrolledWindow {
                        set_vexpand: true,
                        set_hexpand: true,
                        set_propagate_natural_width:true,
                        set_min_content_width: 200,
                        #[local_ref]
                        tunnels_list_box -> gtk::ListBox {}
                    },

                    gtk::Box {

                        append: model.import_button.widget(),

                        gtk::Button {
                            set_label: "Generate Configs",
                            connect_clicked => Self::Input::ShowGenerator,
                        },
                        gtk::Button {
                            set_label: "Documentation",
                            connect_clicked =>
                            Self::Input::OpenUrl(get_doc_url()),

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
                                #[watch]
                                set_sensitive: model.save_button_enabled,
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
                    }
                }
            }
        }
    }

    fn init(
        _counter: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Load custom CSS for toasts
        let css_provider = gtk::CssProvider::new();
        css_provider.load_from_data(
            ".error-toast { background-color: #8b0000; color: white; padding: 8px; border-radius: 6px; } \
             .info-toast { background-color: #1e88e5; color: white; padding: 8px; border-radius: 6px; }"
        );
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().expect("Could not connect to a display."),
            &css_provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let ghaf_pixbuf = pixbuf_from_bytes(GHAF_LOGO);
        let wg_pixbuf = pixbuf_from_bytes(WG_LOGO);
        let tunnels = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |output| match output {
                TunnelOutput::Remove(idx) => Self::Input::RemoveTunnel(idx),

                TunnelOutput::Error(msg) => Self::Input::Error(msg),
            });

        let initial_load_cfg = perform_initial_loading();
        sender.input(initial_load_cfg);

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
                OverviewOutput::FieldsModified => {
                    trace!("FieldsModified");
                    Self::Input::TunnelModified
                }
            });

        let generator =
            GeneratorModel::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    GeneratorOutput::GeneratedHostConfig(cfg) => {
                        /* cfg, set some of fields default */
                        Self::Input::AddTunnel {
                            config: Box::new(cfg),
                            set_default: true,
                        }
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
            toaster: Toaster::default(),
            init_err_buffer: Vec::new(),
            init_complete: false,
            save_button_enabled: false,
        };

        let toast_overlay = model.toaster.overlay_widget();

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

        gtk::glib::idle_add_local_once(move || {
            sender.input(AppMsg::InitComplete);
        });
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            Self::Input::ShowOverview(idx) => {
                self.selected_tunnel_idx = Some(idx);
                trace!("select-Tunnel idx:{}", idx);

                if let Some(tunnel) = self.tunnels.get(idx) {
                    trace!(
                        "select-Tunnel idx:{}, button:{},mark_saved:{}",
                        idx, self.save_button_enabled, tunnel.data.saved
                    );
                    self.overview.emit(OverviewInput::ShowConfig(Box::new(
                        tunnel.data.config.clone(),
                    )));
                    self.save_button_enabled = !tunnel.data.saved;
                }
            }
            Self::Input::AddTunnel {
                config,
                set_default,
            } => {
                let mut tunnels = self.tunnels.guard();

                if tunnels
                    .iter()
                    .any(|t| t.data.config.interface.name == config.interface.name)
                {
                    sender.input(Self::Input::Error(format!(
                        "Tunnel with name {} already exists",
                        config.interface.name.as_ref().unwrap()
                    )));
                    return;
                }

                tunnels.push_back((*config, false));
                trace!("AddTunnel");

                if set_default {
                    self.overview.emit(OverviewInput::SetRoutingScript(None));
                }

                let last_idx = tunnels.len() - 1;
                // Use idle_add to select after UI updates
                let list_box = tunnels.widget().clone();
                gtk::glib::idle_add_local_once(move || {
                    if let Some(row) = list_box.row_at_index(last_idx as i32) {
                        list_box.select_row(Some(&row));
                    }
                });
            }
            Self::Input::RemoveTunnel(idx) => {
                // 1) Lock and inspect the list
                let mut tunnels = self.tunnels.guard();
                if let Some(tunnel) = tunnels.get(idx.current_index()) {
                    let path = tunnel.data.path();

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
                // Read file
                let content = match std::fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(e) => {
                        sender.input(Self::Input::Error(format!(
                            "Failed to read file {}: {}",
                            path.display(),
                            e
                        )));
                        return;
                    }
                };

                // Parse config
                let config = match parse_config(&content) {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        sender.input(Self::Input::Error(format!(
                            "Failed to parse config: {}",
                            e
                        )));
                        return;
                    }
                };

                sender.input(Self::Input::ProcessImportedTunnel(Box::new(config), path));
            }
            Self::Input::ProcessImportedTunnel(mut config, path) => {
                reset_interface_hooks(&mut config);

                if config.interface.name.is_none() {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned);

                    match name {
                        Some(n) => config.interface.name = Some(n),
                        None => {
                            sender.input(Self::Input::Error(format!(
                                "Invalid filename: {}",
                                path.display()
                            )));
                            return;
                        }
                    }
                }

                let Some(ref name) = config.interface.name else {
                    sender.input(Self::Input::Error("Config has no name".into()));
                    return;
                };

                let cfg_path = wireguard_gui::cli::get_configs_dir().join(format!("{}.conf", name));

                if cfg_path.exists() {
                    sender.input(Self::Input::Error(format!(
                        "Config '{}' already exists",
                        name
                    )));
                    return;
                }

                if let Err(e) = write_config_to_path(&config, &cfg_path) {
                    sender.input(Self::Input::Error(format!(
                        "Failed to write config: {}",
                        e
                    )));
                    return;
                }

                sender.input(Self::Input::AddTunnel {
                    config,
                    set_default: false,
                });
            }
            Self::Input::TunnelModified => {
                if !self.init_complete {
                    // Ignore modifications during init
                    return;
                }
                trace!("TunnelModified");

                if let Some(idx) = self.selected_tunnel_idx
                    && let Some(selected_tunnel) = self.tunnels.guard().get_mut(idx)
                {
                    //trace!("TunnelModified- selected_tunnel:{:#?}", selected_tunnel);

                    selected_tunnel.data.saved = false;
                    self.save_button_enabled = !selected_tunnel.data.saved;
                }
            }
            Self::Input::SaveConfigInitiate => {
                self.overview.emit(OverviewInput::CollectTunnel(None));
            }
            Self::Input::ExportConfigInitiate => self
                .export_dialog
                .emit(SaveDialogMsg::SaveAs("my_export.conf".into())),

            Self::Input::SaveConfigFinish(config, path) => {
                let Some(idx) = self.selected_tunnel_idx else {
                    return;
                };
                if let Some(selected_tunnel) = self.tunnels.guard().get_mut(idx) {
                    if selected_tunnel.data.active {
                        sender.input(Self::Input::Error(
                            "Tunnel should be disabled before saving the configuration".into(),
                        ));
                        return;
                    }

                    /* if path is None, it is called by 'Save' function.
                    Otherwise it is called by 'Export' function */
                    let is_save_clicked = path.is_none();
                    let new_tunnel_data = TunnelData::new(*config, false);
                    let save_path = match path {
                        Some(p) if validate_export_path(&p) => p,
                        Some(_) => {
                            sender.input(Self::Input::Error(
                                "Config file can be exported only under 'home' directory"
                                    .into(),
                            ));
                            return;
                        }
                        None => new_tunnel_data.path(),
                    };

                    info!("Saving config file to {}", save_path.display());

                    if let Err(e) =
                        write_config_to_path(&new_tunnel_data.config, &save_path)
                    {
                        sender.input(Self::Input::Error(e.to_string()));
                        return;
                    }
                    sender.input(Self::Input::Info(format!(
                        "Configuration saved to {}",
                        save_path.display()
                    )));

                    sender.input(Self::Input::UpdateTunnel {
                        idx,
                        new_tunnel_data: Box::new(new_tunnel_data),
                        is_save_clicked,
                    });
                }
            }
            Self::Input::UpdateTunnel {
                idx,
                new_tunnel_data,
                is_save_clicked,
            } => {
                if let Some(selected_tunnel) = self.tunnels.guard().get_mut(idx) {
                    if is_save_clicked {
                        selected_tunnel.update_from(*new_tunnel_data);
                    }
                    self.save_button_enabled = !selected_tunnel.data.saved;
                    trace!(
                        "Tunnel idx:{}, button:{},mark_saved:{}",
                        idx, self.save_button_enabled, selected_tunnel.data.saved
                    );
                } else {
                    sender.input(Self::Input::Error(format!(
                        "Tunnel idx cannot be found :{}",
                        idx
                    )));
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
                self.overview.emit(OverviewInput::InitRoutingScripts(s));
            }
            Self::Input::OverviewInitIfaceBindings(b) => {
                self.overview.emit(OverviewInput::InitIfaceBindings(b));
            }
            Self::Input::Error(msg) => {
                debug!("Self::Input::Error : {msg}");
                self.show_error_toast(&msg);
            }
            Self::Input::Info(msg) => {
                debug!("Self::Input::Info : {msg}");
                self.show_info_toast(&msg);
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
                debug!("Init process is completed");
            }
            AppMsg::OpenUrl(url) => {
                // spawn a Tokio task
                sender.oneshot_command(gtk::glib::clone!(
                    #[strong]
                    sender,
                    async move {
                        if let Err(e) = tokio::process::Command::new("xdg-open")
                            .arg(&url)
                            .status()
                            .await
                        {
                            let msg = format!("Failed to open URL '{}': {}", url, e);
                            error!("{}", msg);
                            sender.input(Self::Input::Error(msg));
                        }
                    }
                ));
            }
            AppMsg::InitSyncFinished {
                scripts,
                binding_ifaces,
                loaded_configs,
                initial_errors,
            } => {
                // 1) Push errors
                for err in initial_errors {
                    sender.input(Self::Input::AddInitErrors(err));
                }

                // 2) Send scripts + interfaces to Overview
                sender.input(Self::Input::OverviewInitScripts(scripts.clone()));
                sender.input(Self::Input::OverviewInitIfaceBindings(
                    binding_ifaces.clone(),
                ));

                // 3) Insert loaded configs into tunnels Factory
                let mut guard = self.tunnels.guard();
                for cfg in loaded_configs {
                    guard.push_back((cfg, true));
                }

                debug!("Sync init is completed");
            }
            Self::Input::Ignore => (),
        }
    }
}

enum ToastType {
    Error,
    Info,
}

impl App {
    fn show_toast(&self, msg: &str, toast_type: ToastType) {
        let (icon_name, css_class, timeout, priority) = match toast_type {
            ToastType::Error => ("dialog-error-symbolic", "error-toast", 0, adw::ToastPriority::High),
            ToastType::Info => ("dialog-information-symbolic", "info-toast", 5, adw::ToastPriority::Normal),
        };
    
        // 1) Label
        let label = gtk::Label::new(None);
        label.set_markup(&msg);
        label.set_ellipsize(pango::EllipsizeMode::None);
        label.set_wrap_mode(pango::WrapMode::Word);
        label.set_wrap(true);
        label.set_xalign(0.0);
    
        // 2) Icon
        let image = gtk::Image::from_icon_name(icon_name);
    
        // 3) Horizontal box: left icon, right label
        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 6); // spacing: 6px
        hbox.append(&image);
        hbox.append(&label);
    
        hbox.add_css_class(css_class);
    
        // 4) Toast
        let toast = adw::Toast::builder()
            .custom_title(&hbox)
            .timeout(timeout)
            .priority(priority)
            .build();
    
        self.toaster.add_toast(toast);
    }
    

    fn show_error_toast(&self, msg: &str) {
        self.show_toast(msg, ToastType::Error);
    }

    fn show_info_toast(&self, msg: &str) {
        self.show_toast(msg, ToastType::Info);
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

fn pixbuf_from_bytes(bytes: &[u8]) -> anyhow::Result<gdk_pixbuf::Pixbuf> {
    let loader = gdk_pixbuf::PixbufLoader::new();
    loader.write(bytes).context("PixbufLoader.write error")?;
    loader.close().context("PixbufLoader.close error")?;
    loader
        .pixbuf()
        .context("PixbufLoader returned no pixbuf...")
}
fn perform_initial_loading() -> AppMsg {
    let mut initial_errors = Vec::new();

    // 1. Load routing scripts
    let (scripts, script_err) = extract_scripts_metadata();
    if let Some(err) = script_err {
        initial_errors.push(err);
    }
    debug!("scripts: {:#?}", scripts);

    // 2. Load available interfaces
    let binding_ifaces = get_binding_interfaces();

    // 3. Load existing configs
    let cfgs_result = wireguard_gui::utils::load_existing_configurations();

    let mut loaded_configs = Vec::new();
    match cfgs_result {
        Ok((cfgs, err_opt)) => {
            if let Some(err) = err_opt {
                initial_errors.push(err);
            }

            for mut cfg in cfgs {
                let mut needs_save = false;

                // Validate iface binding
                if let Err(e) = validate_binding_iface(&binding_ifaces, &cfg) {
                    initial_errors.push(e.to_string());
                    needs_save = true;
                }

                // Validate routing script
                if let Err(e) = validate_assign_routing_script(&scripts, &mut cfg) {
                    initial_errors.push(e.to_string());
                    needs_save = true;
                }

                // If modified → save it back
                if needs_save {
                    reset_interface_hooks(&mut cfg);
                    if let Some(name) = &cfg.interface.name {
                        let path = get_configs_dir().join(format!("{name}.conf"));
                        if let Err(e) = write_config_to_path(&cfg, &path) {
                            initial_errors.push(format!("Failed to update {name}: {e}"));
                        }
                    }
                }

                loaded_configs.push(cfg);
            }
        }
        Err(e) => {
            initial_errors.push(format!("Could not load existing configurations: {e:#?}"));
        }
    }

    AppMsg::InitSyncFinished {
        scripts,
        binding_ifaces,
        loaded_configs,
        initial_errors,
    }
}
