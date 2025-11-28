/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use std::{
    io::{self},
    path::PathBuf,
};

use gtk::prelude::*;
use relm4::prelude::*;

use crate::utils::*;
use crate::{cli, config::*};
use getifaddrs::{InterfaceFlags, getifaddrs};
use log::*;
use relm4_components::alert::*;
use std::process::Stdio;
use std::path::Path;
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
    pub saved: bool,
}

impl Tunnel {
    pub fn new(config: WireguardConfig, saved: bool) -> Self {
        let name = config.interface.name.clone().unwrap_or("unknown".into());

        let active = Self::is_wg_iface_running(&name) == NetState::WgQuickUp;

        Self {
            name,
            config,
            active,
            pending_remove: None,
            alert_dialog: None,
            saved,
        }
    }
    pub fn mark_saved(&mut self) {
        self.saved = true;
    }

    pub fn mark_dirty(&mut self) {
        self.saved = false;
    }
    pub fn update_from(&mut self, other: Tunnel) {
        self.active = other.active;
        self.pending_remove = other.pending_remove;
        self.config = other.config;
        self.name = other.name;
        self.mark_saved();
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

    fn is_cfg_valid(&self) -> anyhow::Result<()> {
        if self.config.interface.public_key.is_none() {
            anyhow::bail!("Public key is not defined in interface section");
        }

        if self.config.interface.listen_port.is_none() {
            anyhow::bail!("Listen port is not defined in interface section");
        }

        if self.config.interface.address.is_none() {
            anyhow::bail!("IP address is not defined in interface section");
        }

        if self.config.peers.is_empty() {
            anyhow::bail!("No peers defined");
        }

        if self
            .config
            .peers
            .iter()
            .any(|p| p.public_key.as_deref().is_none_or(|p| p.trim().is_empty()))
        {
            anyhow::bail!("Peer public key is empty");
        }

        Ok(())
    }

    fn is_wg_iface_running(interface: &str) -> NetState {
        let cmd_str = format!("wg show {interface}");

        // Run `wg show <interface>`
        let wg_output = std::process::Command::new("wg")
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
    fn execute_toggle(name: &str, path: &Path) -> anyhow::Result<()> {
        let run_wg_quick = |action: &str| -> Result<(), io::Error> {
            let cmd_str = format!("wg-quick {} {}", action, name);

            let cmd = std::process::Command::new("wg-quick")
                .args([action, path.to_str().unwrap()])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;
            debug!("running cmd: {cmd_str}");
            let (status_code, output) = wait_cmd_with_timeout(cmd, 5, Some(&cmd_str))?;

            if status_code != Some(0) {
                return Err(io::Error::other(format!(
                    "Failed to execute wg-quick {action}: {}",
                    output.trim()
                )));
            }

            Ok(())
        };

        let state = Self::is_wg_iface_running(name);

        match state {
            NetState::IplinkDown => {
                run_wg_quick("down")?;
                run_wg_quick("up")?;
            }
            NetState::WgQuickUp => {
                run_wg_quick("down")?;
            }
            NetState::WgQuickDown => {
                // Validation already done before calling this
                run_wg_quick("up")?;
            }
            _ => anyhow::bail!("Unknown interface state"),
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
#[derive(Debug)]
pub enum TunnelCommandOutput {
    ToggleSuccess(bool), // new active state
    ToggleError(String),
}

#[relm4::factory(pub)]
impl FactoryComponent for Tunnel {
    type Init = (WireguardConfig, bool);
    type Input = TunnelMsg;
    type Output = TunnelOutput;
    type CommandOutput = TunnelCommandOutput;
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

    fn init_model(
        (config, saved): Self::Init,
        _index: &DynamicIndex,
        sender: FactorySender<Self>,
    ) -> Self {
        let mut new_self_cfg = Self::new(config, saved);

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
            // In the Toggle message handler:
            Self::Input::Toggle => {
                // Only validate when activating (not when deactivating)
                if !self.active {
                    if !self.saved {
                        sender.output_sender().emit(TunnelOutput::Error(
                            "You must save the configuration before activating the tunnel.".into(),
                        ));
                        widgets.switch.set_state(false);
                        return;
                    }

                    if let Err(err) = self.is_cfg_valid() {
                        sender
                            .output_sender()
                            .emit(TunnelOutput::Error(err.to_string()));
                        widgets.switch.set_state(false);
                        return;
                    }
                }
                // Capture needed data before moving into async block
                let tunnel_name = self.name.clone();
                let tunnel_path = self.path();
                let current_active = self.active;

                sender.spawn_oneshot_command(move || {
                    match Tunnel::execute_toggle(&tunnel_name, &tunnel_path) {
                        Ok(_) => {
                            debug!("Successfully toggled tunnel: {}", tunnel_name);
                            TunnelCommandOutput::ToggleSuccess(!current_active)
                        }
                        Err(err) => {
                            error!("Error toggling tunnel '{}': {}", tunnel_name, err);
                            TunnelCommandOutput::ToggleError(format!(
                                "Failed to toggle tunnel '{}': {}",
                                tunnel_name, err
                            ))
                        }
                    }
                });
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
    fn update_cmd_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::CommandOutput,
        sender: FactorySender<Self>,
    ) {
        match message {
            TunnelCommandOutput::ToggleSuccess(new_active_state) => {
                self.active = new_active_state;
                widgets.switch.set_state(self.active);
                debug!("connection state: {}", self.active);
            }
            TunnelCommandOutput::ToggleError(err) => {
                sender.output_sender().emit(TunnelOutput::Error(err));
                widgets.switch.set_state(self.active); // Revert switch state
            }
        }
    }
}
