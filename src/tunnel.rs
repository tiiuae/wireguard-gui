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
use std::path::Path;
use std::process::Stdio;
#[derive(PartialEq)]
pub enum NetState {
    IplinkUp = 0x01,
    IplinkDown = 0x02,
    WgQuickUp = 0x04,
    WgQuickDown = 0x08,
}

#[derive(Debug)]
pub struct Tunnel {
    pub data: TunnelData,
    pub pending_remove: Option<DynamicIndex>,
    alert_dialog: Option<Controller<Alert>>,
}

#[derive(Debug)]
pub struct TunnelData {
    pub name: String,
    pub config: WireguardConfig,
    pub active: bool,
    pub saved: bool,
}

impl TunnelData {
    pub fn new(config: WireguardConfig, saved: bool) -> Self {
        let name = config.interface.name.clone().unwrap_or("unknown".into());
        let active: bool = Tunnel::is_wg_iface_running(&name) == NetState::WgQuickUp;

        Self {
            name,
            config,
            active,
            saved,
        }
    }
    pub fn path(&self) -> PathBuf {
        if self.name != "unknown" {
            return cli::get_configs_dir().join(format!("{}.conf", self.name));
        }
        PathBuf::new()
    }
}

impl Tunnel {
    pub fn new(data: TunnelData) -> Self {
        Self {
            data,
            pending_remove: None,
            alert_dialog: None,
        }
    }
    pub fn update_from(&mut self, other: TunnelData) {
        self.data.active = other.active;
        self.data.config = other.config;
        self.data.name = other.name;
        self.data.saved = true;
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
        let iface = &self.data.config.interface;

        // Check required interface fields
        let required_fields = [
            ("Public key", iface.public_key.as_deref()),
            ("Listen port", iface.listen_port.as_deref()),
            ("IP address", iface.address.as_deref()),
        ];

        for (name, value) in required_fields {
            if value.is_none() {
                anyhow::bail!("{name} is not defined in interface section");
            }
        }

        // Check peers exist
        let peers = &self.data.config.peers;
        if peers.is_empty() {
            anyhow::bail!("No peers defined");
        }

        // Check peer public keys
        if peers
            .iter()
            .any(|p| p.public_key.as_deref().is_none_or(|v| v.trim().is_empty()))
        {
            anyhow::bail!("Peer public key is empty");
        }

        // Binding interface check
        if iface.has_script_bind_iface && iface.binding_iface.is_none() {
            anyhow::bail!("Binding interface cannot be selected as 'None'");
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

    /// Toggle the `WireGuard` interface using wireguard-tools.
    fn execute_toggle(name: &str, path: &Path) -> anyhow::Result<()> {
        let run_wg_quick = |action: &str| -> anyhow::Result<()> {
            let cmd_str = format!("wg-quick {} {}", action, name);

            let cmd = std::process::Command::new("wg-quick")
                .arg(action)
                .arg(path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| anyhow::anyhow!("Failed to spawn wg-quick: {}", e))?;

            debug!("running cmd: {cmd_str}");
            let (status_code, output) = wait_cmd_with_timeout(cmd, 5, Some(&cmd_str))
                .map_err(|e| anyhow::anyhow!("Command timeout or IO error: {}", e))?;

            if status_code != Some(0) {
                anyhow::bail!("Failed to execute wg-quick {}: {}", action, output.trim());
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
                set_active: self.data.active,
                connect_state_notify => Self::Input::Toggle,
            },

            gtk::Label {
                set_label: &self.data.name,
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
        let data = TunnelData::new(config, saved);
        let mut new_tunnel = Tunnel::new(data);
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

        new_tunnel.alert_dialog = Some(alert_dialog);
        new_tunnel
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
                if !self.data.active {
                    if !self.data.saved {
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
                let tunnel_name = self.data.name.clone();
                let tunnel_path = self.data.path();
                let current_active = self.data.active;

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
                self.data.active = new_active_state;
                widgets.switch.set_state(self.data.active);
                debug!("connection state: {}", self.data.active);
            }
            TunnelCommandOutput::ToggleError(err) => {
                trace!("Emitting TunnelOutput::Error to main app: {}", err);
                sender.output_sender().emit(TunnelOutput::Error(err));
                widgets.switch.set_state(self.data.active); // Revert switch state
            }
        }
    }
}
