use std::path::PathBuf;

use gtk::prelude::*;
use relm4::*;
use relm4_components::open_dialog::*;

pub struct TunnelsModel {
    open_dialog: Controller<OpenDialog>,
    tunnels: Vec<()>,
    current_tunnel_idx: usize,
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

    /// Delete currently selected tunnel configuration.
    DeleteCurrent,
}

#[relm4::component(pub)]
impl SimpleComponent for TunnelsModel {
    type Init = ();
    type Input = TunnelsInput;
    type Output = TunnelsOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                gtk::ScrolledWindow {
                    set_vexpand: true,
                    gtk::Viewport {
                        set_child: Some(gtk::ListBox::new()).as_ref(),
                    }
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

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                gtk::Frame::new(Some("Interface:")) {
                    gtk::Grid {
                        attach[0, 0, 1, 2] = &gtk::Label::new(Some("hello")),
                    }
                },

                gtk::Frame::new(Some("Peer:")) {},
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

        let model = TunnelsModel {
            open_dialog,
            tunnels: vec![],
            current_tunnel_idx: 0,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            Self::Input::ImportRequest => self.open_dialog.emit(OpenDialogMsg::Open),
            Self::Input::ImportResponse(path) => {
                dbg!(path);
            },
            Self::Input::Ignore => (),

            Self::Input::DeleteCurrent => {
                if self.current_tunnel_idx < self.tunnels.len() {
                    self.tunnels.remove(self.current_tunnel_idx);
                    self.current_tunnel_idx = self.current_tunnel_idx.saturating_sub(1);

                }
            },
            // Self::Input:: => {
            //     // self.mode = mode;
            // }
        }
    }
}
