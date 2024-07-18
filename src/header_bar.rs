use gtk::prelude::*;
use relm4::*;

pub struct HeaderModel;

#[derive(Debug)]
pub enum HeaderOutput {
    Tunnels,
    Logs,
}

#[relm4::component(pub)]
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
