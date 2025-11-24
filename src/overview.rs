/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/

use std::path::PathBuf;
// use gtk::prelude::*;
use crate::config::*;
use crate::peer::*;
use crate::{cli, utils};
use log::{debug, error};
use relm4::factory::{DynamicIndex, FactoryVecDeque};
use relm4::{gtk::prelude::*, prelude::*};
use std::cell::RefCell;
use std::rc::Rc;
pub struct OverviewModel {
    interface: Interface,
    peers: FactoryVecDeque<PeerComp>,
    routing_scripts: Rc<RefCell<Vec<RoutingScripts>>>,
    routing_scripts_list: gtk::StringList,
    binding_ifaces_list: gtk::StringList,
    binding_ifaces_enabled: bool,
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
    BindingIfaces,
}

#[derive(Debug)]
pub enum OverviewInput {
    CollectTunnel(Option<PathBuf>),
    ShowConfig(Box<WireguardConfig>),
    RemovePeer(DynamicIndex),
    AddPeer,
    SetInterface(InterfaceSetKind, Option<String>),
    SetRoutingScript(Option<RoutingScripts>),
    InitRoutingScripts(Vec<RoutingScripts>),
    InitIfaceBindings(Vec<String>),
}

#[derive(Debug)]
pub enum OverviewOutput {
    SaveConfig(Box<WireguardConfig>, Option<PathBuf>),
    AddInitErrors(String),
    Error(String),
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
                        set_label: "Binding Network Inteface:",
                        set_halign: gtk::Align::Start,
                    },
                    #[name = "binding_ifaces"]
                    attach[1, 8, 1, 1] = &gtk::DropDown {
                        set_model: Some(&model.binding_ifaces_list),
                        #[watch]
                        set_sensitive: model.binding_ifaces_enabled,
                        #[watch]
                        set_selected: {
                            model.interface.binding_iface
                                .as_ref()
                                .and_then(|iface| {
                                    // Find position in the list (which already includes "None" at 0)
                                    (0..model.binding_ifaces_list.n_items())
                                        .find(|&i| {
                                            model.binding_ifaces_list
                                                .string(i)
                                                .map(|s| s.as_str() == iface.as_str())
                                                .unwrap_or(false)
                                        })
                                })
                                .unwrap_or(gtk::INVALID_LIST_POSITION) // Default to "None" at index 0
                        },
                        connect_selected_notify[sender] => move |dropdown| {
                            if let Some(list) = dropdown.model().and_then(|m| m.downcast::<gtk::StringList>().ok()) {
                                if let Some(item) = list.string(dropdown.selected()) {
                                    let iface_name = if item.to_string() == "None" {
                                        None
                                    } else {
                                        Some(item.to_string())
                                    };
                                    dropdown.set_tooltip_text(Some("Replaces %bindIface in routing script files".into()));

                                    sender.input(Self::Input::SetInterface(
                                        InterfaceSetKind::BindingIfaces,
                                       iface_name
                                    ));
                                }
                            }
                        },
                    },


                    attach[0, 9, 1, 1] = &gtk::Label {
                        set_label: "Routing Scripts:",
                        set_halign: gtk::Align::Start,
                    },
                    #[name = "routing_scripts"]
                    attach[1, 9, 1, 1] = &gtk::DropDown {
                        set_model: Some(&model.routing_scripts_list),
                        #[watch]
                        set_selected: {
                            if let Some(name) = &model.interface.routing_script_name {
                                model.routing_scripts.borrow()
                                    .iter()
                                    .position(|s| s.name == *name)
                                    .map(|i| i as u32 + 1)
                                    .unwrap_or(gtk::INVALID_LIST_POSITION)
                            } else {
                                gtk::INVALID_LIST_POSITION
                            }
                        },
                        connect_selected_notify[sender, scripts = Rc::clone(&model.routing_scripts)] => move |dropdown| {
                            if let Some(list) = dropdown.model().and_then(|m| m.downcast::<gtk::StringList>().ok()) {
                                if let Some(item) = list.string(dropdown.selected()) {
                                    const MAX_CONTENT_CHARS: usize = 400;
                                    let name = item.to_string();
                                    // Borrow once for the search
                                    let scripts_ref = scripts.borrow();
                                    let selected_script = if name == "None" {
                                        None
                                    } else {
                                        scripts_ref.iter().find(|s| s.name == name).cloned()
                                    };

                                    let tooltip_text = selected_script
                                        .as_ref()
                                        .map(|s| s.content.chars().take(MAX_CONTENT_CHARS).collect::<String>())
                                        .unwrap_or_else(|| "No script selected".to_string());
                                    dropdown.set_tooltip_text(Some(&tooltip_text));

                                    sender.input(OverviewInput::SetRoutingScript(selected_script));


                                }
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
            binding_ifaces_list: Default::default(),
            routing_scripts: Rc::new(Vec::new().into()),
            routing_scripts_list: Default::default(),
            binding_ifaces_enabled: true,
        };

        model.replace_peers(config.peers);

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            Self::Input::CollectTunnel(path) => {
                let cfg = WireguardConfig {
                    interface: self.interface.clone(),
                    peers: self.peers.iter().map(|p| p.peer.clone()).collect(),
                };

                sender
                    .output_sender()
                    .emit(Self::Output::SaveConfig(Box::new(cfg), path));
            }
            Self::Input::ShowConfig(config) => {
                let WireguardConfig { interface, peers } = *config;
                self.interface = interface;
                // Disable/Enable dropdown when script requires bind interface
                self.binding_ifaces_enabled = self.interface.has_script_bind_iface;
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
            Self::Input::SetRoutingScript(selected_script) => {
                if let Some(script) = selected_script {
                    self.interface.has_script_bind_iface = script.has_bind_interface;
                    // Disable/Enable dropdown when script requires bind interface
                    self.binding_ifaces_enabled = script.has_bind_interface;

                    self.interface.routing_script_name = Some(script.name);

                    if self.binding_ifaces_enabled {
                        // Ensure binding_iface exists
                        let bind_iface = match &self.interface.binding_iface {
                            Some(iface) if !iface.is_empty() => iface,
                            _ => {
                                sender.output_sender().emit(OverviewOutput::Error(
                                    "No binding interface selected".to_string(),
                                ));
                                return;
                            }
                        };
                        // Helper to replace %bindIface in Option<String> ya da daha önceden atama yapıldıysa onu ata
                        let replace_bindiface = |field: &Option<String>| -> Option<String> {
                            field.as_ref().map(|s| s.replace("%bindIface", bind_iface))
                        };
                        // Assign all script fields with %bindIface replaced
                        self.interface.pre_up = replace_bindiface(&script.pre_up);
                        self.interface.pre_down = replace_bindiface(&script.pre_down);
                        self.interface.post_up = replace_bindiface(&script.post_up);
                        self.interface.post_down = replace_bindiface(&script.post_down);
                    } else {
                        self.interface.pre_up = script.pre_up;
                        self.interface.pre_down = script.pre_down;
                        self.interface.post_up = script.post_up;
                        self.interface.post_down = script.post_down;
                    }
                } else {
                    self.interface.pre_up = None;
                    self.interface.pre_down = None;
                    self.interface.post_up = None;
                    self.interface.post_down = None;
                    self.interface.routing_script_name = None;
                    self.interface.has_script_bind_iface = false;
                }

                // TODO: enforce user to click save
            }
            Self::Input::InitIfaceBindings(b) => {
                // Build the new list including "None"
                let new_items: Vec<&str> = std::iter::once("None")
                    .chain(b.iter().map(|s| s.as_str()))
                    .collect();

                // Insert items at the start, no need to remove anything
                self.binding_ifaces_list.splice(0, 0, &new_items);
            }
            Self::Input::InitRoutingScripts(s) => {
                // Update the GTK list
                let new_items: Vec<&str> = std::iter::once("None")
                    .chain(s.iter().map(|s| s.name.as_str()))
                    .collect();
                self.routing_scripts_list.splice(0, 0, &new_items);

                // Update Rc contents
                self.routing_scripts.borrow_mut().clear();
                self.routing_scripts.borrow_mut().extend(s);
            }
            Self::Input::SetInterface(kind, value) => match kind {
                InterfaceSetKind::Name => self.interface.name = value,
                InterfaceSetKind::Address => {
                    if !utils::is_ip_valid(value.as_deref()) {
                        sender
                            .output_sender()
                            .emit(Self::Output::Error("Invalid IP address".to_string()));
                        return;
                    }
                    self.interface.address = value;
                }
                InterfaceSetKind::ListenPort => self.interface.listen_port = value,
                InterfaceSetKind::PrivateKey => {
                    let Some(private_key) = value.clone() else {
                        return;
                    };
                    let public_key = match utils::generate_public_key(private_key.clone()) {
                        Ok(key) => key,
                        Err(e) => {
                            sender
                                .output_sender()
                                .emit(Self::Output::Error(e.to_string()));
                            return;
                        }
                    };
                    self.interface.public_key = Some(public_key);
                    self.interface.private_key = value;

                    debug!("public key: {:?}", self.interface.public_key);
                }
                InterfaceSetKind::Dns => self.interface.dns = value,
                InterfaceSetKind::Table => self.interface.table = value,
                InterfaceSetKind::Mtu => self.interface.mtu = value,
                InterfaceSetKind::BindingIfaces => {
                    //TODO: enforce user for saving or save automatically
                    // TODO: change binding interface in current script
                    self.interface.binding_iface = value;
                    sender.input(Self::Input::SetRoutingScript(
                        self.interface
                            .routing_script_name
                            .as_ref()
                            .and_then(|name| {
                                self.routing_scripts
                                    .borrow()
                                    .iter()
                                    .find(|s| &s.name == name)
                                    .cloned()
                            }),
                    ));
                }
            },
        }
    }
}
