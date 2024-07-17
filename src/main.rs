use gtk::prelude::{
    *, // ButtonExt, GtkWindowExt, ToggleButtonExt, WidgetExt,
};
use gtk::Align;
use relm4::*;

pub mod components;

use components::log::*;

#[derive(Default)]
struct TunnelsModel {
    tunnels: Vec<()>,
    current_tunnel_idx: usize,
}

#[derive(Debug)]
enum TunnelsOutput {
    /// Passthrough logs from executed commands to logs tab.
    LogEntry(String),
}

#[relm4::component]
impl SimpleComponent for TunnelsModel {
    type Init = ();
    type Input = ();
    type Output = TunnelsOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_valign: Align::Fill,
            // set_vexpand: true,
            // set_hexpand: true,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                // set_vexpand: true,
                // set_hexpand: true,

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
                        set_label: "Add Tunnel"
                    },

                    gtk::Button {
                        set_label: "Import Tunnel"
                    },

                    gtk::Separator {

                    },

                    gtk::Button::from_icon_name("bin"),
                }
            },

            gtk::Box {

            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = TunnelsModel::default();
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }
}

struct HeaderModel;

#[derive(Debug)]
enum HeaderOutput {
    Tunnels,
    Logs,
}

#[relm4::component]
impl SimpleComponent for HeaderModel {
    type Init = ();
    type Input = ();
    type Output = HeaderOutput;

    view! {
        gtk::HeaderBar {
            #[wrap(Some)]
            set_title_widget = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_homogeneous: true,

                gtk::Label {
                    set_text: "Wireguard"
                },

                gtk::Label {
                    add_css_class: "subtitle",
                    set_text: "not connected"
                }
            },


            pack_start = &gtk::Box {
                add_css_class: "linked",
                set_orientation: gtk::Orientation::Horizontal,

                #[name = "group"]
                gtk::ToggleButton::with_label("Tunnels") {
                    set_active: true,
                    connect_toggled[sender] => move |btn| {
                        if btn.is_active() {
                            sender.output(HeaderOutput::Tunnels).unwrap()
                        }
                    },
                },

                gtk::ToggleButton::with_label("Logs") {
                    set_group: Some(&group),
                    connect_toggled[sender] => move |btn| {
                        if btn.is_active() {
                            sender.output(HeaderOutput::Logs).unwrap()
                        }
                    },
                },
            },
            pack_end = &gtk::MenuButton {
                #[wrap(Some)]
                set_popover: popover = &gtk::Popover {
                    set_position: gtk::PositionType::Right,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        // // TODO: Add settings
                        // gtk::Button {
                        //     set_label: "Increment counter",
                        //     // connect_clicked => Msg::Increment,
                        // },

                        gtk::Label {
                            set_label: "Version: 0.1.0",
                        },
                    },
                },
            },

        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = HeaderModel;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }
}

#[derive(Debug)]
enum AppMode {
    Tunnels,
    Logs,
}

#[derive(Debug)]
enum AppMsg {
    SetMode(AppMode),
}

struct AppModel {
    mode: AppMode,
    header: Controller<HeaderModel>,
    tunnels: Controller<TunnelsModel>,
    logs: Controller<LogsModel>
}

#[relm4::component]
impl SimpleComponent for AppModel {
    type Init = AppMode;
    type Input = AppMsg;
    type Output = ();

    view! {
        main_window = gtk::Window {
            set_default_width: 500,
            set_default_height: 250,
            set_titlebar: Some(model.header.widget()),
            set_halign: gtk::Align::Fill,
            set_valign: gtk::Align::Fill,
            set_hexpand: true,
            set_vexpand: true,

            #[wrap(Some)]
            set_child = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                // set_halign: gtk::Align::Fill,
                set_hexpand: true,
                set_vexpand: true,
                match &model.mode {
                    AppMode::Tunnels => model.tunnels.widget().clone(),
                    AppMode::Logs => model.logs.widget().clone(),
                }
            }
        }
    }

    fn init(
        params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let header: Controller<HeaderModel> =
            HeaderModel::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    HeaderOutput::Tunnels => AppMsg::SetMode(AppMode::Tunnels),
                    HeaderOutput::Logs => AppMsg::SetMode(AppMode::Logs),
                });

        let logs = LogsModel::builder().launch(root.width()).detach();

        let tunnels = TunnelsModel::builder()
            .launch(())
            .forward(logs.sender(), |msg| {
                match msg {
                    TunnelsOutput::LogEntry(s) => LogsInput::LogEntry(s)
                }
            });

        let model = AppModel {
            mode: params,
            header,
            tunnels,
            logs
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AppMsg::SetMode(mode) => {
                self.mode = mode;
            }
        }
    }
}

fn main() {
    let relm = RelmApp::new("ewlm4.test.components");
    relm.run::<AppModel>(AppMode::Tunnels);
}
