/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use std::{
    io::{self},
    path::PathBuf,
    process::Command,
};

use gtk::prelude::*;
use relm4::prelude::*;

use crate::utils::*;
use crate::{cli, config::*};
use getifaddrs::{getifaddrs, InterfaceFlags};
use log::*;
use relm4_components::alert::*;
use std::net::SocketAddr;
use std::str::FromStr;

#[derive(PartialEq)]
pub enum NetState {
    IplinkUp = 0x01,
    IplinkDown = 0x02,
    WgQuickUp = 0x04,
    WgQuickDown = 0x08,
}

#[derive(Debug)]
pub struct Tunnel {
    pub name: String,
    pub config: WireguardConfig,
    pub active: bool,
    pub pending_remove: Option<DynamicIndex>,
    alert_dialog: Option<Controller<Alert>>,
}

impl Tunnel {
    pub fn new(config: WireguardConfig) -> Self {
        let name = config.interface.name.clone().unwrap_or("unknown".into());

        let active = Self::is_wg_iface_running(&name) == NetState::WgQuickUp;

        Self {
            name,
            config,
            active,
            pending_remove: None,
            alert_dialog: None,
        }
    }
    pub fn update_from(&mut self, other: Tunnel) {
        self.active = other.active;
        self.pending_remove = other.pending_remove;
        self.config = other.config;
        self.name = other.name;
    }
    fn is_interface_up(interface_name: &str) -> Result<bool, std::io::Error> {
        let ifaddrs =
            getifaddrs().map_err(|_| std::io::Error::other("Failed to get interfaces"))?;

        for interface in ifaddrs {
            if interface.name == interface_name {
                return Ok(interface.flags.contains(InterfaceFlags::UP)
                    && interface.flags.contains(InterfaceFlags::RUNNING));
            }
        }

        Ok(false)
    }

    fn is_wg_iface_running(interface: &str) -> NetState {
        let cmd_str = format!("wg show {interface}");

        // Run `wg show <interface>`
        let wg_output = Command::new("wg")
            .arg("show")
            .arg(interface)
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to execute wg show");

        debug!("running cmd: {cmd_str}");

        if !wait_cmd_with_timeout(wg_output, 5, None)
            .is_ok_and(|(code, output)| code == Some(0) && !output.is_empty())
        {
            info!("Interface {} is not running", interface);
            return NetState::WgQuickDown;
        }

        if !Self::is_interface_up(interface).unwrap_or(false) {
            return NetState::IplinkDown;
        }
        info!("Interface {} is running", interface);
        NetState::WgQuickUp
    }

    pub fn path(&self) -> PathBuf {
        cli::get_configs_dir().join(format!("{}.conf", self.name))
    }

    /// Toggle the `WireGuard` interface using wireguard-tools.
    pub fn try_toggle(&mut self) -> Result<(), io::Error> {
        let is_endpoint_valid = |config: &WireguardConfig| -> Result<(), io::Error> {
            for peer in &config.peers {
                if let Some(endpoint) = peer.endpoint.as_ref() {
                    // Try to parse the endpoint into a SocketAddr
                    if SocketAddr::from_str(endpoint).is_err() {
                        return Err(io::Error::other("Invalid endpoint format"));
                    }
                }
            }
            Ok(())
        };

        // Helper closure to run a command and check its success
        let run_wg_quick = |action: &str| -> Result<(), io::Error> {
            let cmd_str = format!("wg-quick {} {}", action, self.name);

            let cmd = Command::new("wg-quick")
                .args([action, &self.name])
                .spawn()?;
            debug!("running cmd: {cmd_str}");
            if !wait_cmd_with_timeout(cmd, 3, Some(&cmd_str)).is_ok_and(|(code, _)| code == Some(0))
            {
                error!("Failed to execute wg-quick {} {}", action, &self.name);
                return Err(io::Error::other(format!(
                    "Failed to execute wg-quick {action}",
                )));
            }
            Ok(())
        };

        let state = Self::is_wg_iface_running(self.name.as_str());

        // Check if the endpoint is valid before wireguard inteface is up
        if state != NetState::WgQuickUp {
            is_endpoint_valid(&self.config)?;
        }

        match state {
            NetState::IplinkDown => {
                run_wg_quick("down")?;
                run_wg_quick("up")?;
            }
            NetState::WgQuickUp => {
                run_wg_quick("down")?;
            }
            NetState::WgQuickDown => {
                run_wg_quick("up")?;
            }
            _ => return Err(io::Error::other("Unknown interface state")),
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum TunnelMsg {
    Toggle,
    Remove(DynamicIndex),
    RemoveConfirmed,
    Ignore,
}

#[derive(Debug)]
pub enum TunnelOutput {
    Remove(DynamicIndex),
    Error(String),
}

#[relm4::factory(pub)]
impl FactoryComponent for Tunnel {
    type Init = WireguardConfig;
    type Input = TunnelMsg;
    type Output = TunnelOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        #[name(root)]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 5,

            #[name(switch)]
            gtk::Switch {
                set_active: self.active,
                connect_state_notify => Self::Input::Toggle,
            },

            gtk::Label {
                set_label: &self.name,
            },

            gtk::Button::with_label("Remove") {
               // connect_clicked => Self::Input::Remove,

                connect_clicked[sender, index] => move |_| {
                    sender.input(Self::Input::Remove(index.clone()));
                }
            },
        }
    }

    fn init_model(config: Self::Init, _index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        let mut new_self_cfg = Self::new(config);
        let alert_dialog = Alert::builder()
            .launch(AlertSettings {
                text: Some(String::from("Are you sure to remove this tunnel?")),
                confirm_label: Some(String::from("Remove")),
                cancel_label: Some(String::from("Cancel")),
                is_modal: true,
                destructive_accept: true,
                ..Default::default()
            })
            .forward(sender.input_sender(), move |response| match response {
                AlertResponse::Confirm => Self::Input::RemoveConfirmed,
                _ => Self::Input::Ignore,
            });

        new_self_cfg.alert_dialog = Some(alert_dialog);

        new_self_cfg
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: relm4::FactorySender<Self>,
    ) {
        match msg {
            Self::Input::Toggle => {
                match self.try_toggle() {
                    Ok(_) => self.active = !self.active,
                    Err(err) => sender
                        .output_sender()
                        .emit(Self::Output::Error(err.to_string())),
                };
                debug!("connection state: {}", self.active);
                widgets.switch.set_state(self.active);
            }
            Self::Input::Remove(index) => {
                self.pending_remove = Some(index.clone());
                if let Some(alert_dialog) = self.alert_dialog.as_ref() {
                    alert_dialog.emit(AlertMsg::Show);
                }
            }
            Self::Input::RemoveConfirmed => {
                let index = self.pending_remove.take().unwrap();
                sender.output(Self::Output::Remove(index)).unwrap();
            }
            Self::Input::Ignore => {
                // Ignore the message
            }
        }
    }
}
