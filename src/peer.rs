use std::fmt;

use gtk::prelude::*;
use relm4::prelude::*;

use crate::config::*;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Default, Clone)]
pub struct PeerComp {
    pub peer: Peer,
}

impl PeerComp {
    pub fn new(peer: Peer) -> Self {
        Self {
            peer
        }
    }
}

pub enum PeerInput {
    Modify(Box<dyn FnOnce(&mut Peer)>)
}

impl fmt::Debug for PeerInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PeerInput::Modify(_) => f.write_str("SetCurrentTunnel(<fn>)"),
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
    type Input = PeerInput;
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
                    set_text: &self.peer.name.clone().unwrap_or("unknown".into()),
                    connect_changed[sender] => move |l| {
                        let new: String = l.text().trim().into();
                        sender.input(Self::Input::Modify(Box::new(|p| {
                            p.name = (new != "unknown").then_some(new);
                        })));
                    },
                },

                attach[0, 1, 1, 1] = &gtk::Label {
                    set_label: "AllowedIPs:",
                    set_halign: gtk::Align::Start,
                },
                attach[1, 1, 1, 1] = &gtk::EditableLabel {
                    set_text: &self.peer.allowed_ips.clone().unwrap_or("unknown".into()),
                    connect_changed[sender] => move |l| {
                        let new: String = l.text().trim().into();
                        sender.input(Self::Input::Modify(Box::new(|p| {
                            p.allowed_ips = (new != "unknown").then_some(new);
                        })));
                    },
                },

                attach[0, 2, 1, 1] = &gtk::Label {
                    set_label: "Endpoint:",
                    set_halign: gtk::Align::Start,
                },
                attach[1, 2, 1, 1] = &gtk::EditableLabel {
                    set_text: &self.peer.endpoint.clone().unwrap_or("unknown".into()),
                    connect_changed[sender] => move |l| {
                        let new: String = l.text().trim().into();
                        sender.input(Self::Input::Modify(Box::new(|p| {
                            p.endpoint = (new != "unknown").then_some(new);
                        })));
                    },
                },

                attach[0, 3, 1, 1] = &gtk::Label {
                    set_label: "PublicKey:",
                    set_halign: gtk::Align::Start,
                },
                attach[1, 3, 1, 1] = &gtk::EditableLabel {
                    set_text: &self.peer.public_key.clone().unwrap_or("unknown".into()),
                    connect_changed[sender] => move |l| {
                        let new: String = l.text().trim().into();
                        sender.input(Self::Input::Modify(Box::new(|p| {
                            p.public_key = (new != "unknown").then_some(new);
                        })));
                    },
                },

                attach[0, 4, 1, 1] = &gtk::Label {
                    set_label: "PersistentKeepalive:",
                    set_halign: gtk::Align::Start,
                },
                attach[1, 4, 1, 1] = &gtk::EditableLabel {
                    set_text: &self.peer.persistent_keepalive.clone().unwrap_or("unknown".into()),
                    connect_changed[sender] => move |l| {
                        let new: String = l.text().trim().into();
                        sender.input(Self::Input::Modify(Box::new(|p| {
                            p.persistent_keepalive = (new != "unknown").then_some(new);
                        })));
                    },
                },
            }
        }
    }

    fn init_model(peer_config: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self::new(peer_config)
    }

    fn update(&mut self, msg: Self::Input, _sender: relm4::FactorySender<Self>) {
        match msg {
            Self::Input::Modify(f) => {
                f(&mut self.peer);
                dbg!(&self.peer);
            },
        }
    }
}
