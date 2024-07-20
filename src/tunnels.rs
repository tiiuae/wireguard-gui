use std::path::PathBuf;

use gtk::prelude::*;
use relm4::{
    prelude::*,
    typed_view::{
        list::{RelmListItem, TypedListView},
        TypedListItem,
    },
};
use relm4_components::open_dialog::*;

use crate::config::*;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Tunnel {
    pub name: String,
    pub config: WireguardConfig,
    pub active: bool,
}

impl Tunnel {
    pub fn new(config: WireguardConfig) -> Self {
        let cfg = config.clone();
        Self {
            active: false,
            name: config.interface.name.unwrap_or("unknown".into()),
            config: cfg,
        }
    }

    /// Toggles activation of a tunnel.
    ///
    /// Returns either new state, or error string from the command output.
    pub fn toggle(&mut self) -> Result<bool, String> {
        if self.active {
            self.active = false;
            Ok(self.active)
        } else {
            // TODO: Run command to activate tunnel.
            todo!();
            self.active = true;
        }
    }
}

impl Drop for Tunnel {
    fn drop(&mut self) {
        // TODO: Disconnect if connected
    }
}

// TODO: Add activity indication
impl RelmListItem for Tunnel {
    type Root = gtk::Box;
    // For now let entry be only label with name.
    type Widgets = gtk::Label;

    fn setup(_item: &gtk::ListItem) -> (gtk::Box, Self::Widgets) {
        relm4::view! {
            my_box = gtk::Box {
                #[name = "label"]
                gtk::Label,
            }
        }

        let widgets = label;

        (my_box, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        let label = widgets;
        label.set_label(&self.name);
    }
}

pub struct TunnelsModel {
    open_dialog: Controller<OpenDialog>,
    tunnel_list_view_wrapper: TypedListView<Tunnel, gtk::SingleSelection>,
}

impl TunnelsModel {
    pub fn selected(&self) -> Option<TypedListItem<Tunnel>> {
        self.tunnel_list_view_wrapper
            .get(self.tunnel_list_view_wrapper.selection_model.selected())
    }
}

#[derive(Debug)]
pub enum TunnelsOutput {
    /// Passthrough logs from executed commands to logs tab.
    LogEntry(String),
}

/// Actions on tunnels list.
#[derive(Debug)]
pub enum TunnelsInput {
    // Dialog actions
    ImportRequest,
    ImportResponse(PathBuf),
    Ignore,

    // Message with index of selected item
    UpdateCurrentShowedTunnel(u32),

    /// Delete currently selected tunnel configuration.
    DeleteCurrent,
}

#[relm4::component(pub)]
impl SimpleComponent for TunnelsModel {
    type Init = ();
    type Input = TunnelsInput;
    type Output = TunnelsOutput;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                gtk::ScrolledWindow {
                    set_vexpand: true,

                    #[local_ref]
                    tunnels_view -> gtk::ListView {}
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,

                    gtk::Button {
                        set_label: "Add Tunnel",
                        connect_clicked[sender] => move |_| {
                            sender.output(Self::Output::LogEntry("new tunnel were added\n".into())).unwrap()
                        },
                    },

                    gtk::Button {
                        set_label: "Import Tunnel",
                        connect_clicked => Self::Input::ImportRequest,
                    },

                    gtk::Separator::default(),

                    gtk::Button::from_icon_name("edit-delete") {
                        connect_clicked => Self::Input::DeleteCurrent
                    },
                }
            },

            // TODO: Move to separate component.
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                gtk::Frame::new(Some("Interface")) {

                    // TODO: Change labels to input entry.
                    gtk::Grid {
                        set_column_spacing: 5,

                        attach[0, 0, 1, 1] = &gtk::Label::new(Some("Name:")),
                        #[name = "interface_name_label"]
                        attach[1, 0, 1, 1] = &gtk::EditableLabel {
                            connect_changed[sender] => move |l| {
                                // TODO: Update state
                                // l.
                                // &model.selected();
                            },
                            // #[watch]
                            // set_label: {
                            //     model
                            //         .selected()
                            //         .map(|x| x.borrow().config.interface.name.clone())
                            //         .flatten()
                            //         .unwrap_or("unknown".into())
                            //         .as_str()
                            // },
                        },

                        attach[0, 1, 1, 1] = &gtk::Label::new(Some("Address:")),
                        attach[1, 1, 1, 1] = &gtk::Label{
                            #[watch]
                            set_label: {
                                model
                                    .selected()
                                    .map(|x| x.borrow().config.interface.address.clone())
                                    .flatten()
                                    .unwrap_or("unknown".into())
                                    .as_str()
                            },
                        },

                        attach[0, 2, 1, 1] = &gtk::Label::new(Some("Public Key:")),
                        #[name = "public_key"]
                        attach[1, 2, 1, 1] = &gtk::Label{
                            #[watch]
                            set_label: {
                                "unknown"
                                // model
                                //     .selected()
                                //     // TODO: Calculate public key
                                //     .map(|x| x.borrow().config.interface.private_key.clone())
                                //     .flatten()
                                //     .unwrap_or("unknown".into())
                                //     .as_str()
                            },
                        },
                        attach[2, 2, 2, 1] = &gtk::Button::with_label("Generate Private Key") {
                            // TODO: Generate new key on click and paste into next line public key label.
                        },


                        attach[0, 3, 1, 1] = &gtk::Label::new(Some("Listen port:")),
                        attach[1, 3, 1, 1] = &gtk::Label{
                            #[watch]
                            set_label: {
                                model
                                    .selected()
                                    .map(|x| x.borrow().config.interface.listen_port.clone())
                                    .flatten()
                                    .unwrap_or("unknown".into())
                                    .as_str()
                            },
                        },

                    }
                },

                // TODO: Set custom label widget with label and buttons to create new peer below or delete current.
                gtk::Frame::new(Some("Peer")) {
                    #[name = "peer_grid"]
                    gtk::Grid {

                    }
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let open_dialog = OpenDialog::builder()
            .transient_for_native(&root)
            // TODO: Configure properly
            .launch(OpenDialogSettings::default())
            .forward(sender.input_sender(), |response| match response {
                OpenDialogResponse::Accept(path) => Self::Input::ImportResponse(path),
                OpenDialogResponse::Cancel => Self::Input::Ignore,
            });

        let tunnel_list_view_wrapper: TypedListView<Tunnel, gtk::SingleSelection> =
            TypedListView::with_sorting();

        tunnel_list_view_wrapper
            .selection_model
            .connect_selected_notify(gtk::glib::clone!(@strong sender => move |s| {
                sender.input(TunnelsInput::UpdateCurrentShowedTunnel(s.selected()));
            }));

        // tunnel_list_view_wrapper.append(Tunnel::default());

        let model = TunnelsModel {
            open_dialog,
            tunnel_list_view_wrapper,
        };

        let tunnels_view = &model.tunnel_list_view_wrapper.view;

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            Self::Input::ImportRequest => self.open_dialog.emit(OpenDialogMsg::Open),
            Self::Input::ImportResponse(path) => {
                // TODO: Show modal window with error.
                let file_content = std::fs::read_to_string(path);
                let res = file_content.map(|c| parse_config(&c));
                let Ok(Ok(config)) = dbg!(res) else {
                    return;
                };

                self.tunnel_list_view_wrapper.append(Tunnel::new(config));
            }
            Self::Input::Ignore => (),

            Self::Input::UpdateCurrentShowedTunnel(idx) => {
                let Some(item) = self.tunnel_list_view_wrapper.get(idx) else {
                    return;
                };
                let it = item.borrow_mut();
                println!("{}:{}", it.name, idx)
            }

            Self::Input::DeleteCurrent => {
                let selected_item = self.tunnel_list_view_wrapper.selection_model.selected();
                self.tunnel_list_view_wrapper.remove(selected_item);
            }
        }
    }
}
