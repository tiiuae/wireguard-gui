/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use gtk::prelude::*;
use relm4::prelude::*;

use crate::config::*;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Default, Clone)]
pub struct PeerComp {
    pub peer: Peer,
}

impl PeerComp {
    pub fn new(peer: Peer) -> Self {
        Self { peer }
    }
}

#[derive(Debug)]
pub enum PeerSetKind {
    Name,
    AllowedIps,
    Endpoint,
    PublicKey,
    PersistentKeepalive,
}

#[derive(Debug)]
pub enum PeerInput {
    Set(PeerSetKind, Option<String>),
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
                    #[watch]
                    set_text: get_value(&self.peer.name),
                    connect_editing_notify[sender] => move |l| {
                        if !l.is_editing() {
                            let new: String = l.text().trim().into();
                            sender.input(Self::Input::Set(PeerSetKind::Name, (new != "unknown").then_some(new)));
                        }
                    },
                },

                attach[0, 1, 1, 1] = &gtk::Label {
                    set_label: "AllowedIPs:",
                    set_halign: gtk::Align::Start,
                },
                attach[1, 1, 1, 1] = &gtk::EditableLabel {
                    set_text: get_value(&self.peer.allowed_ips),
                    connect_editing_notify[sender] => move |l| {
                        if !l.is_editing() {
                            let new: String = l.text().trim().into();
                            sender.input(Self::Input::Set(PeerSetKind::AllowedIps, (new != "unknown").then_some(new)));
                        }
                    },
                },

                attach[0, 2, 1, 1] = &gtk::Label {
                    set_label: "Endpoint:",
                    set_halign: gtk::Align::Start,
                },
                attach[1, 2, 1, 1] = &gtk::EditableLabel {
                    set_text: get_value(&self.peer.endpoint),
                    connect_editing_notify[sender] => move |l| {
                        if !l.is_editing() {
                            let new: String = l.text().trim().into();
                            sender.input(Self::Input::Set(PeerSetKind::Endpoint, (new != "unknown").then_some(new)));
                        }
                    },
                },

                attach[0, 3, 1, 1] = &gtk::Label {
                    set_label: "PublicKey:",
                    set_halign: gtk::Align::Start,
                },
                attach[1, 3, 1, 1] = &gtk::EditableLabel {
                    set_text: get_value(&self.peer.public_key),
                    connect_editing_notify[sender] => move |l| {
                        if !l.is_editing() {
                            let new: String = l.text().trim().into();
                            sender.input(Self::Input::Set(PeerSetKind::PublicKey, (new != "unknown").then_some(new)));
                        }
                    },
                },

                attach[0, 4, 1, 1] = &gtk::Label {
                    set_label: "PersistentKeepalive:",
                    set_halign: gtk::Align::Start,
                },
                attach[1, 4, 1, 1] = &gtk::EditableLabel {
                    set_text: get_value(&self.peer.persistent_keepalive),
                    connect_editing_notify[sender] => move |l| {
                        if !l.is_editing() {
                            let new: String = l.text().trim().into();
                            sender.input(Self::Input::Set(PeerSetKind::PersistentKeepalive, (new != "unknown").then_some(new)));
                        }
                    },
                },
            }
        }
    }

    fn init_model(
        peer_config: Self::Init,
        _index: &DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        Self::new(peer_config)
    }

    fn update(&mut self, msg: Self::Input, _sender: relm4::FactorySender<Self>) {
        match msg {
            Self::Input::Set(k, value) => match k {
                PeerSetKind::Name => self.peer.name = value,
                PeerSetKind::AllowedIps => self.peer.allowed_ips = value,
                PeerSetKind::Endpoint => self.peer.endpoint = value,
                PeerSetKind::PublicKey => self.peer.public_key = value,
                PeerSetKind::PersistentKeepalive => self.peer.persistent_keepalive = value,
            },
        }
    }
}
