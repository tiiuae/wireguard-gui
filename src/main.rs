use std::path::PathBuf;

use gtk::prelude::*;
use relm4::factory::{DynamicIndex, FactoryVecDeque};
use relm4::prelude::*;
use relm4_components::open_button::{OpenButton, OpenButtonSettings};
use relm4_components::open_dialog::OpenDialogSettings;
use relm4_components::save_dialog::*;

use wireguard_gui::{config::*, overview::*, tunnel::*};

struct App {
    tunnels: FactoryVecDeque<Tunnel>,
    selected_tunnel_idx: Option<usize>,
    overview: Controller<OverviewModel>,
    import_button: Controller<OpenButton>,
    export_dialog: Controller<SaveDialog>,
}

#[derive(Debug)]
enum AppMsg {
    ShowOverview(usize),
    AddTunnel(Box<WireguardConfig>),
    RemoveTunnel(DynamicIndex),
    ImportTunnel(PathBuf),
    ExportStart,
    ExportTunnel(PathBuf),
    SaveConfigInitiate,
    SaveConfigFinish(Box<WireguardConfig>),
    AddPeer,
    // Generate keypair, assign it to tunnel and show new tunnel.
    GenerateKeypair,
    Error(String),
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

                        #[local_ref]
                        tunnels_list_box -> gtk::ListBox {}
                    },

                    gtk::Box {
                        gtk::Button {
                            set_label: "Add Tunnel",
                            connect_clicked => Self::Input::AddTunnel(Box::default()),
                        },

                        append: model.import_button.widget()
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

                            #[name = "export_button"]
                            gtk::Button {
                                set_label: "Export",
                                connect_clicked => Self::Input::ExportStart,
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
            },
            Err(err) => {
                eprintln!("Could not load existing configurations: {:#?}", err);
            },
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

        let export_dialog = SaveDialog::builder()
            .transient_for_native(&root)
            .launch(SaveDialogSettings {
                accept_label: String::from("Export"),
                cancel_label: String::from("Cancel"),
                create_folders: true,
                is_modal: true,
                filters: vec![{
                    let filter = gtk::FileFilter::new();
                    filter.add_mime_type("application/x-tar");
                    // filter.
                    // filter.add_pattern("*.tar");
                    filter
                }],
            })
            .forward(sender.input_sender(), |response| match response {
                SaveDialogResponse::Accept(path) => Self::Input::ExportTunnel(path),
                SaveDialogResponse::Cancel => Self::Input::Ignore,
            });

        let overview = OverviewModel::builder()
            .launch(WireguardConfig::default())
            .forward(sender.input_sender(), |msg| match msg {
                OverviewOutput::GenerateKeypair => AppMsg::GenerateKeypair,
                OverviewOutput::SaveConfig(config) => AppMsg::SaveConfigFinish(config),
            });

        let model = App {
            tunnels,
            selected_tunnel_idx: None,
            import_button,
            export_dialog,
            overview,
        };

        let tunnels_list_box = model.tunnels.widget();

        tunnels_list_box.connect_row_selected(gtk::glib::clone!(@strong sender => move |_, row| {
            if let Some(lbr) = row {
                sender.input_sender().emit(AppMsg::ShowOverview(lbr.index().try_into().unwrap()));
            }
        }));

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
            Self::Input::ExportStart => {
                let Some(name) = self.selected_tunnel_idx.and_then(|idx| {
                    self.tunnels
                        .guard()
                        .get(idx)
                        .map(|selected_tunnel| selected_tunnel.name.clone())
                }) else {
                    sender
                        .input_sender()
                        .emit(Self::Input::Error("No tunnel selected to export.".into()));
                    return;
                };

                self.export_dialog
                    .emit(SaveDialogMsg::SaveAs(format!("{name}.tar")));
            }
            Self::Input::ExportTunnel(path) => {
                let Some(mut tunnel) = self
                    .selected_tunnel_idx
                    .and_then(|idx| self.tunnels.guard().get(idx).cloned())
                else {
                    sender
                        .input_sender()
                        .emit(Self::Input::Error("No tunnel selected to export.".into()));
                    return;
                };

                if let Err(err) = tunnel.write_configs_to_path(path) {
                    sender.input_sender().emit(Self::Input::Error(format!(
                        "Config creation error: {:#?}",
                        err
                    )));
                }
            }
            Self::Input::SaveConfigInitiate => self.overview.emit(OverviewInput::CollectTunnel),
            Self::Input::SaveConfigFinish(tunnel) => {
                let Some(idx) = self.selected_tunnel_idx else {
                    return;
                };
                if let Some(selected_tunnel) = self.tunnels.guard().get_mut(idx) {
                    *selected_tunnel = Tunnel::new(*tunnel);
                }
            }
            Self::Input::AddPeer => {
                self.overview.emit(OverviewInput::AddPeer);
            }
            Self::Input::GenerateKeypair => todo!(),
            Self::Input::Error(msg) => {
                // TODO: Emit modal window on error
                dbg!(msg);
            }
            Self::Input::Ignore => (),
        }
    }
}

fn main() {
    #[cfg(release)]
    if !nix::unistd::Uid::effective().is_root() {
        panic!("You must run this executable with root permissions");
    }

    let app = RelmApp::new("relm4.ghaf.wireguard-gui");
    app.run::<App>(());
}
