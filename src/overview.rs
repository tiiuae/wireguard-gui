/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/

// use gtk::prelude::*;
use relm4::factory::{DynamicIndex, FactoryVecDeque};
use relm4::{gtk::prelude::*, prelude::*};

use crate::config::*;
use crate::peer::*;
use crate::utils;
use log::debug;
pub struct OverviewModel {
    interface: Interface,
    peers: FactoryVecDeque<PeerComp>,
}

impl OverviewModel {
    pub fn replace_peers(&mut self, peers: Vec<Peer>) {
        let mut ps = self.peers.guard();
        ps.clear();

        for peer in peers {
            ps.push_back(peer);
        }
    }
}

#[derive(Debug)]
pub enum InterfaceSetKind {
    Name,
    Address,
    ListenPort,
    PrivateKey,
    Dns,
    Table,
    Mtu,
    PreUp,
    PostUp,
    PreDown,
    PostDown,
}

#[derive(Debug)]
pub enum OverviewInput {
    CollectTunnel,
    ShowConfig(Box<WireguardConfig>),
    RemovePeer(DynamicIndex),
    AddPeer,
    SetInterface(InterfaceSetKind, Option<String>),
}

#[derive(Debug)]
pub enum OverviewOutput {
    SaveConfig(Box<WireguardConfig>),
}

#[relm4::component(pub)]
impl SimpleComponent for OverviewModel {
    type Init = WireguardConfig;
    type Input = OverviewInput;
    type Output = OverviewOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,

            gtk::Frame::new(Some("Interface:")) {
                gtk::Grid {
                    set_row_spacing: 5,
                    set_column_spacing: 5,
                    set_margin_all: 5,

                    attach[0, 0, 1, 1] = &gtk::Label {
                        set_label: "# Name:",
                        set_halign: gtk::Align::Start,
                    },
                    #[name = "name"]
                    attach[1, 0, 1, 1] = &gtk::EditableLabel {
                        #[watch]
                        set_text: get_value(&model.interface.name),
                        connect_editing_notify[sender] => move |l| {
                            if !l.is_editing() {
                                let new: String = l.text().trim().into();
                                sender.input(Self::Input::SetInterface(InterfaceSetKind::Name, (new != "unknown").then_some(new)));
                            }
                        },
                    },

                    attach[0, 1, 1, 1] = &gtk::Label {
                        set_label: "Address:",
                        set_halign: gtk::Align::Start,
                    },
                    #[name = "address"]
                    attach[1, 1, 1, 1] = &gtk::EditableLabel {
                        #[watch]
                        set_text: get_value(&model.interface.address),
                        connect_editing_notify[sender] => move |l| {
                            if !l.is_editing() {
                                let new: String = l.text().trim().into();
                                sender.input(Self::Input::SetInterface(InterfaceSetKind::Address, (new != "unknown").then_some(new)));
                            }
                        },
                    },

                    attach[0, 2, 1, 1] = &gtk::Label {
                        set_label: "ListenPort:",
                        set_halign: gtk::Align::Start,
                    },
                    #[name = "listen_port"]
                    attach[1, 2, 1, 1] = &gtk::EditableLabel {
                        #[watch]
                        set_text: get_value(&model.interface.listen_port),
                        connect_editing_notify[sender] => move |l| {
                            if !l.is_editing() {
                                let new: String = l.text().trim().into();
                                sender.input(Self::Input::SetInterface(InterfaceSetKind::ListenPort, (new != "unknown").then_some(new)));
                            }
                        },
                    },
                         // TODO: Just show omitted
                         attach[0, 3, 1, 1] = &gtk::Label {
                            set_label: "PublicKey:",
                            set_halign: gtk::Align::Start,
                        },
                        #[name = "public_key"]
                        attach[1, 3, 1, 1] = &gtk::Label {
                            #[watch]
                            set_text: get_value(&model.interface.public_key),
                            set_selectable: true,
                            set_xalign: 0.0,
                            // connect_editing_notify[sender] => move |l| {
                            //     if !l.is_editing() {
                            //         let new: String = l.text().trim().into();
                            //         sender.input(Self::Input::SetInterface(InterfaceSetKind::PrivateKey, (new != "unknown").then_some(new)));
                            //     }
                            // },
                        },

                    // TODO: Just show omitted
                    attach[0, 4, 1, 1] = &gtk::Label {
                        set_label: "PrivateKey:",
                        set_halign: gtk::Align::Start,
                    },
                    #[name = "private_key"]
                    attach[1, 4, 1, 1] = &gtk::EditableLabel {
                        #[watch]
                        set_text: get_value(&model.interface.private_key),
                        connect_editing_notify[sender] => move |l| {
                            if !l.is_editing() {
                                let new: String = l.text().trim().into();
                                sender.input(Self::Input::SetInterface(InterfaceSetKind::PrivateKey, (new != "unknown").then_some(new)));
                            }
                        },
                    },

                    attach[0, 5, 1, 1] = &gtk::Label {
                        set_label: "DNS:",
                        set_halign: gtk::Align::Start,
                    },
                    #[name = "dns"]
                    attach[1, 5, 1, 1] = &gtk::EditableLabel {
                        #[watch]
                        set_text: get_value(&model.interface.dns),
                        connect_editing_notify[sender] => move |l| {
                            if !l.is_editing() {
                                let new: String = l.text().trim().into();
                                sender.input(Self::Input::SetInterface(InterfaceSetKind::Dns, (new != "unknown").then_some(new)));
                            }
                        },
                    },

                    attach[0, 6, 1, 1] = &gtk::Label {
                        set_label: "Table:",
                        set_halign: gtk::Align::Start,
                    },
                    #[name = "table"]
                    attach[1, 6, 1, 1] = &gtk::EditableLabel {
                        #[watch]
                        set_text: get_value(&model.interface.table),
                        connect_editing_notify[sender] => move |l| {
                            if !l.is_editing() {
                                let new: String = l.text().trim().into();
                                sender.input(Self::Input::SetInterface(InterfaceSetKind::Table, (new != "unknown").then_some(new)));
                            }
                        },
                    },

                    attach[0, 7, 1, 1] = &gtk::Label {
                        set_label: "MTU:",
                        set_halign: gtk::Align::Start,
                    },
                    #[name = "mtu"]
                    attach[1, 7, 1, 1] = &gtk::EditableLabel {
                        #[watch]
                        set_text: get_value(&model.interface.mtu),
                        connect_editing_notify[sender] => move |l| {
                            if !l.is_editing() {
                                let new: String = l.text().trim().into();
                                sender.input(Self::Input::SetInterface(InterfaceSetKind::Mtu, (new != "unknown").then_some(new)));
                            }
                        },
                    },

                    attach[0, 8, 1, 1] = &gtk::Label {
                        set_label: "PreUp:",
                        set_halign: gtk::Align::Start,
                    },
                    #[name = "pre_up"]
                    attach[1, 8, 1, 1] = &gtk::EditableLabel {
                        #[watch]
                        set_text: get_value(&model.interface.pre_up),
                        connect_editing_notify[sender] => move |l| {
                            if !l.is_editing() {
                                let new: String = l.text().trim().into();
                                sender.input(Self::Input::SetInterface(InterfaceSetKind::PreUp, (new != "unknown").then_some(new)));
                            }
                        },
                    },

                    attach[0, 9, 1, 1] = &gtk::Label {
                        set_label: "PostUp:",
                        set_halign: gtk::Align::Start,
                    },
                    #[name = "post_up"]
                    attach[1, 9, 1, 1] = &gtk::EditableLabel {
                        #[watch]
                        set_text: get_value(&model.interface.post_up),
                        connect_editing_notify[sender] => move |l| {
                            if !l.is_editing() {
                                let new: String = l.text().trim().into();
                                sender.input(Self::Input::SetInterface(InterfaceSetKind::PostUp, (new != "unknown").then_some(new)));
                            }
                        },
                    },

                    attach[0, 10, 1, 1] = &gtk::Label {
                        set_label: "PreDown:",
                        set_halign: gtk::Align::Start,
                    },
                    #[name = "pre_down"]
                    attach[1, 10, 1, 1] = &gtk::EditableLabel {
                        #[watch]
                        set_text: get_value(&model.interface.pre_down),
                        connect_editing_notify[sender] => move |l| {
                            if !l.is_editing() {
                                let new: String = l.text().trim().into();
                                sender.input(Self::Input::SetInterface(InterfaceSetKind::PreDown, (new != "unknown").then_some(new)));
                            }
                        },
                    },

                    attach[0, 11, 1, 1] = &gtk::Label {
                        set_label: "PostDown:",
                        set_halign: gtk::Align::Start,
                    },
                    #[name = "post_down"]
                    attach[1, 11, 1, 1] = &gtk::EditableLabel {
                        #[watch]
                        set_text: get_value(&model.interface.post_down),
                        connect_editing_notify[sender] => move |l| {
                            if !l.is_editing() {
                                let new: String = l.text().trim().into();
                                sender.input(Self::Input::SetInterface(InterfaceSetKind::PostDown, (new != "unknown").then_some(new)));
                            }
                        },
                    },
                }
            },

            append: model.peers.widget()
        }
    }

    fn init(
        config: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let peers = FactoryVecDeque::builder()
            .launch(gtk::Box::new(gtk::Orientation::Vertical, 5))
            .forward(sender.input_sender(), |output| match output {
                PeerOutput::Remove(idx) => Self::Input::RemovePeer(idx),
            });

        let mut model = Self {
            interface: config.interface,
            peers,
        };

        model.replace_peers(config.peers);

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            Self::Input::CollectTunnel => {
                let cfg = WireguardConfig {
                    interface: self.interface.clone(),
                    peers: self.peers.iter().map(|p| p.peer.clone()).collect(),
                };
                sender
                    .output_sender()
                    .emit(Self::Output::SaveConfig(Box::new(cfg)));
            }
            Self::Input::ShowConfig(config) => {
                let WireguardConfig { interface, peers } = *config;
                self.interface = interface;
                self.replace_peers(peers);
            }
            Self::Input::RemovePeer(idx) => {
                let mut peers = self.peers.guard();
                peers.remove(idx.current_index());
            }
            Self::Input::AddPeer => {
                let mut peers = self.peers.guard();
                peers.push_back(Peer::default());
            }
            Self::Input::SetInterface(kind, value) => match kind {
                InterfaceSetKind::Name => self.interface.name = value,
                InterfaceSetKind::Address => self.interface.address = value,
                InterfaceSetKind::ListenPort => self.interface.listen_port = value,
                InterfaceSetKind::PrivateKey => {
                    //TODO: this should be removed in next release
                    self.interface.public_key = Some(
                        utils::generate_public_key(value.clone().unwrap_or_default())
                            .unwrap_or("Wrong private key".to_string()),
                    );
                    debug!("public key: {:?}", self.interface.public_key);
                    self.interface.private_key = value
                }
                InterfaceSetKind::Dns => self.interface.dns = value,
                InterfaceSetKind::Table => self.interface.table = value,
                InterfaceSetKind::Mtu => self.interface.mtu = value,
                InterfaceSetKind::PreUp => self.interface.pre_up = value,
                InterfaceSetKind::PostUp => self.interface.post_up = value,
                InterfaceSetKind::PreDown => self.interface.pre_down = value,
                InterfaceSetKind::PostDown => self.interface.post_down = value,
            },
        }
    }
}
