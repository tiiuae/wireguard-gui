/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/

use std::path::PathBuf;
// use gtk::prelude::*;
use crate::config::*;
use crate::peer::*;
use crate::utils;
use crate::utils::MutOptionExt;
use log::{debug, trace};
use relm4::factory::{DynamicIndex, FactoryVecDeque};
use relm4::{gtk::prelude::*, prelude::*};
use std::cell::RefCell;
use std::rc::Rc;

const DROPDOWN_NONE_INDEX: u32 = 0;
const DROPDOWN_NONE_STR: &str = "None";
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
    PeerFieldsModified,
    SetGeneratedKeys {
        pub_key: Option<String>,
        priv_key: Option<String>,
    },
}

#[derive(Debug)]
pub enum OverviewOutput {
    SaveConfig(Box<WireguardConfig>, Option<PathBuf>),
    AddInitErrors(String),
    FieldsModified,
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
                                                .is_some_and(|s| &s == iface)
                                        })
                                })
                                .unwrap_or(DROPDOWN_NONE_INDEX) // Default to "None" at index 0
                        },
                        connect_selected_notify[sender] => move |dropdown| {
                            if let Some(list) = dropdown.model().and_downcast::<gtk::StringList>()
                                && let Some(item) = list.string(dropdown.selected()) {
                                    let iface_name = if item == DROPDOWN_NONE_STR {
                                        None
                                    } else {
                                        Some(item.to_string())
                                    };
                                    dropdown.set_tooltip_text(Some("Replaces %bindIface in routing script files"));

                                    sender.input(Self::Input::SetInterface(
                                        InterfaceSetKind::BindingIfaces,
                                       iface_name
                                    ));
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
                            /* 0. index = "None" */
                            model.interface
                            .routing_script_name
                            .as_ref()
                            .and_then(|name| {
                                model.routing_scripts
                                    .borrow()
                                    .iter()
                                    .position(|s| s.name == *name)
                            })
                            .map_or(DROPDOWN_NONE_INDEX, |i| i as u32 + 1)

                        },
                        connect_selected_notify[sender, scripts = Rc::clone(&model.routing_scripts)] => move |dropdown| {
                            if let Some(list) = dropdown.model().and_downcast::<gtk::StringList>()
                                && let Some(item) = list.string(dropdown.selected()) {
                                    const MAX_CONTENT_CHARS: usize = 400;
                                    let name = item.to_string();
                                    let selected_script = if name == "None" {
                                        None
                                    } else {
                                        scripts.borrow().iter().find(|s| s.name == name).cloned()
                                    };

                                    let tooltip_text = selected_script
                                    .as_ref()
                                    .map_or("No script selected", |s| &s.content[0..MAX_CONTENT_CHARS.min(s.content.len())]);

                                    dropdown.set_tooltip_text(Some(tooltip_text));

                                    sender.input(OverviewInput::SetRoutingScript(selected_script));


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
                PeerOutput::FieldsModified => {
                    trace!("Peer FieldsModified");
                    Self::Input::PeerFieldsModified
                }
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
                trace!("show-config: {:#?}", self.interface);
            }
            Self::Input::RemovePeer(idx) => {
                let mut peers = self.peers.guard();
                peers.remove(idx.current_index());
                // notify parent that the overview has unsaved changes
                sender.output_sender().emit(Self::Output::FieldsModified);
            }
            Self::Input::PeerFieldsModified => {
                trace!("PeerFieldsModified");
                // notify parent that the overview has unsaved changes
                sender.output_sender().emit(Self::Output::FieldsModified);
            }
            Self::Input::AddPeer => {
                let mut peers = self.peers.guard();
                peers.push_back(Peer::default());
                // notify parent that the overview has unsaved changes
                trace!("Addpeer");

                sender.output_sender().emit(Self::Output::FieldsModified);
            }
            Self::Input::SetRoutingScript(selected_script) => {
                trace!("SetRoutingScript");
                /* Other models call the SetRoutingScript during the init.
                That's why it changes even if the user does not change */
                let is_changed = self.interface.routing_script_name.as_ref()
                    != selected_script.as_ref().map(|s| &s.name);
                if let Some(script) = selected_script {
                    trace!(
                        "set routing script : {},{:#?}",
                        script.name,
                        self.interface.routing_script_name.as_ref()
                    );

                    let script_routing_hooks = script.routing_hooks;
                    self.interface.has_script_bind_iface = script_routing_hooks.has_bind_interface;
                    // Disable/Enable dropdown when script requires bind interface
                    self.binding_ifaces_enabled = script_routing_hooks.has_bind_interface;
                    self.interface.routing_script_name = Some(script.name);
                    self.interface.fwmark = script_routing_hooks.fwmark;

                    if self.binding_ifaces_enabled {
                        // Ensure binding_iface exists
                        let Some(bind_iface) = self
                            .interface
                            .binding_iface
                            .as_ref()
                            .filter(|iface| !iface.is_empty())
                        else {
                            sender.output_sender().emit(OverviewOutput::Error(
                                "No binding interface selected".to_string(),
                            ));
                            return;
                        };
                        // Helper to replace %bindIface in Option<String>
                        let replace_bindiface = |field: &Option<String>| {
                            field.as_ref().map(|s| s.replace("%bindIface", bind_iface))
                        };
                        // Assign all script fields with %bindIface replaced
                        self.interface.pre_up = replace_bindiface(&script_routing_hooks.pre_up);
                        self.interface.pre_down = replace_bindiface(&script_routing_hooks.pre_down);
                        self.interface.post_up = replace_bindiface(&script_routing_hooks.post_up);
                        self.interface.post_down =
                            replace_bindiface(&script_routing_hooks.post_down);
                    } else {
                        self.interface.pre_up = script_routing_hooks.pre_up;
                        self.interface.pre_down = script_routing_hooks.pre_down;
                        self.interface.post_up = script_routing_hooks.post_up;
                        self.interface.post_down = script_routing_hooks.post_down;
                    }
                } else {
                    self.interface.pre_up = None;
                    self.interface.pre_down = None;
                    self.interface.post_up = None;
                    self.interface.post_down = None;
                    self.interface.routing_script_name = None;
                    self.interface.fwmark = None;
                    self.interface.has_script_bind_iface = false;
                    self.binding_ifaces_enabled = false;
                }
                if is_changed {
                    // notify parent that the overview has unsaved changes
                    sender.output_sender().emit(Self::Output::FieldsModified);
                }
            }
            Self::Input::InitIfaceBindings(b) => {
                // Build the new list including "None"
                let new_items: Vec<&str> = std::iter::once(DROPDOWN_NONE_STR)
                    .chain(b.iter().map(|s| s.as_str()))
                    .collect();

                // Insert items at the start, no need to remove anything
                self.binding_ifaces_list.splice(0, 0, &new_items);
            }
            Self::Input::InitRoutingScripts(s) => {
                // Update the GTK list
                let new_items: Vec<&str> = std::iter::once(DROPDOWN_NONE_STR)
                    .chain(s.iter().map(|s| s.name.as_str()))
                    .collect();
                self.routing_scripts_list.splice(0, 0, &new_items);

                // Update Rc contents
                self.routing_scripts.replace(s);
            }
            Self::Input::SetGeneratedKeys { pub_key, priv_key } => {
                self.interface.public_key = pub_key;
                self.interface.private_key = priv_key;

                debug!(
                    "public key: {}",
                    self.interface.public_key.as_deref().unwrap_or("")
                );
            }
            Self::Input::SetInterface(kind, value) => {
                let mut is_changed = false;
                match kind {
                    InterfaceSetKind::Name => is_changed = self.interface.name.update(value),
                    InterfaceSetKind::Address => {
                        if let Some(ref ip) = value
                            && !utils::is_ip_valid(Some(ip))
                        {
                            sender
                                .output_sender()
                                .emit(Self::Output::Error("Invalid IP address".to_string()));
                            return;
                        }
                        is_changed = self.interface.address.update(value)
                    }
                    InterfaceSetKind::ListenPort => {
                        is_changed = self.interface.listen_port.update(value)
                    }
                    InterfaceSetKind::PrivateKey => {
                        let Some(private_key) = value.clone() else {
                            return;
                        };
                        sender.spawn_oneshot_command(gtk::glib::clone!(
                            #[strong]
                            sender,
                            move || {
                                let public_key =
                                    match utils::generate_public_key(private_key.clone()) {
                                        Ok(k) => k,
                                        Err(e) => {
                                            sender
                                                .output_sender()
                                                .emit(Self::Output::Error(e.to_string()));
                                            return;
                                        }
                                    };
                                sender.input(Self::Input::SetGeneratedKeys {
                                    pub_key: Some(public_key),
                                    priv_key: Some(private_key),
                                });
                            }
                        ));
                    }
                    InterfaceSetKind::Dns => is_changed = self.interface.dns.update(value),
                    InterfaceSetKind::Table => is_changed = self.interface.table.update(value),
                    InterfaceSetKind::Mtu => is_changed = self.interface.mtu.update(value),
                    InterfaceSetKind::BindingIfaces => {
                        is_changed = self.interface.binding_iface.update(value);
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
                }

                if is_changed {
                    trace!("SetInterface: changed = true");
                    sender.output_sender().emit(Self::Output::FieldsModified);
                } else {
                    trace!("SetInterface: no change");
                }
            }
        }
    }
}
