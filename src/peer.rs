use gtk::prelude::*;
use relm4::prelude::*;

use crate::config::*;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Default, Clone)]
pub struct PeerComp {
    peer: Peer,
}

impl PeerComp {
    pub fn new(peer: Peer) -> Self {
        Self {
            peer
        }
    }
}

#[derive(Debug)]
pub enum PeerOutput {
    Remove(DynamicIndex),
}

#[relm4::factory(pub)]
impl FactoryComponent for PeerComp {
    type Init = Peer;
    type Input = ();
    type Output = PeerOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        gtk::Frame /* ::new(Some("Peer:")) */ {
            #[wrap(Some)]
            set_label_widget = &gtk::Box {
                set_spacing: 10,

                gtk::Button::with_label("Remove") {
                    connect_clicked[sender, index] => move |_| {
                        sender.output(Self::Output::Remove(index.clone())).unwrap();
                    }
                },

                gtk::Label {
                    set_label: "Peer:"
                }
            },

            gtk::Grid {
                set_row_spacing: 5,
                set_column_spacing: 5,
                set_margin_all: 5,

                attach[0, 0, 1, 1] = &gtk::Label {
                    set_label: "# Name:",
                    set_halign: gtk::Align::Start,
                },
                attach[1, 0, 1, 1] = &gtk::EditableLabel {
                    #[watch]
                    set_text: &self.peer.name.clone().unwrap_or("unknown".into()),
                },

                attach[0, 1, 1, 1] = &gtk::Label {
                    set_label: "AllowedIPs:",
                    set_halign: gtk::Align::Start,
                },
                attach[1, 1, 1, 1] = &gtk::EditableLabel {
                    #[watch]
                    set_text: &self.peer.allowed_ips.clone().unwrap_or("unknown".into()),
                },

                attach[0, 2, 1, 1] = &gtk::Label {
                    set_label: "Endpoint:",
                    set_halign: gtk::Align::Start,
                },
                attach[1, 2, 1, 1] = &gtk::EditableLabel {
                    #[watch]
                    set_text: &self.peer.endpoint.clone().unwrap_or("unknown".into()),
                },

                attach[0, 3, 1, 1] = &gtk::Label {
                    set_label: "PublicKey:",
                    set_halign: gtk::Align::Start,
                },
                attach[1, 3, 1, 1] = &gtk::EditableLabel {
                    #[watch]
                    set_text: &self.peer.public_key.clone().unwrap_or("unknown".into()),
                },

                attach[0, 4, 1, 1] = &gtk::Label {
                    set_label: "PersistentKeepalive:",
                    set_halign: gtk::Align::Start,
                },
                attach[1, 4, 1, 1] = &gtk::EditableLabel {
                    #[watch]
                    set_text: &self.peer.persistent_keepalive.clone().unwrap_or("unknown".into()),
                },
            }
        }
    }

    fn init_model(peer_config: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self::new(peer_config)
    }
}
