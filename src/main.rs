use std::path::PathBuf;

use gtk::prelude::*;
use relm4::factory::{DynamicIndex, FactoryVecDeque};
use relm4::prelude::*;
use relm4_components::open_button::{OpenButton, OpenButtonSettings};
use relm4_components::open_dialog::OpenDialogSettings;

use wireguard_gui::config::*;
use wireguard_gui::tunnel::*;

struct App {
    tunnels: FactoryVecDeque<Tunnel>,

    import_button: Controller<OpenButton>,
}

#[derive(Debug)]
enum AppMsg {
    AddTunnel(Box<WireguardConfig>),
    RemoveTunnel(DynamicIndex),
    ImportTunnel(PathBuf),
    ExportTunnel,
    SaveTunnel,
    Error(String),
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

            gtk::Grid {
                set_row_spacing: 5,
                set_column_spacing: 5,
                set_margin_all: 5,

                attach[0, 0, 1, 1] = &gtk::ScrolledWindow {
                    set_vexpand: true,
                    // set_hexpand: true,

                    #[local_ref]
                    tunnels_list_box -> gtk::ListBox {}
                },

                attach[0, 1, 1, 1] = &gtk::Box {
                    gtk::Button {
                        set_label: "Add Tunnel",
                        connect_clicked => Self::Input::AddTunnel(Box::default()),
                    },

                    append: model.import_button.widget()
                },


                #[name = "config_overview"]
                attach[1, 0, 1, 1] = &gtk::Box {
                    set_vexpand: true,
                    set_hexpand: true,
                },

                attach[1, 1, 1, 1] = &gtk::CenterBox {
                    #[wrap(Some)]
                    set_end_widget = &gtk::Box {
                        gtk::Button {
                            set_label: "Save",
                            connect_clicked => Self::Input::SaveTunnel,
                        },

                        gtk::Button {
                            set_label: "Export",
                            connect_clicked => Self::Input::ExportTunnel,
                        },
                    }
                }
            },
        }
    }

    fn init(
        _counter: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let tunnels = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |output| match output {
                TunnelOutput::Remove(idx) => Self::Input::RemoveTunnel(idx),

                TunnelOutput::Error(msg) => Self::Input::Error(msg),
            });

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

        let model = App {
            tunnels,
            import_button,
        };

        let tunnels_list_box = model.tunnels.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
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
                let file_content = std::fs::read_to_string(path);
                let res = file_content.map(|c| parse_config(&c));

                let Ok(Ok(config)) = res else {
                    sender.input(Self::Input::Error(format!("{:#?}", res)));
                    return;
                };

                sender.input(Self::Input::AddTunnel(Box::new(config)));
            }
            Self::Input::ExportTunnel => todo!(),
            Self::Input::SaveTunnel => todo!(),
            Self::Input::Error(msg) => {
                // TODO: Emit modal window on error
                dbg!(msg);
            }
        }
    }
}

fn main() {
    let app = RelmApp::new("relm4.ghaf.wireguard-gui");
    app.run::<App>(());
}
