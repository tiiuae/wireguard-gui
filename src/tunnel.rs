use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use gtk::prelude::*;
use relm4::prelude::*;

use crate::config::*;
use crate::utils::*;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Default, Clone)]
pub struct Tunnel {
    pub name: String,
    pub config: WireguardConfig,
    pub active: bool,
}

impl Tunnel {
    pub fn new(config: WireguardConfig) -> Self {
        let name = config.interface.name.clone().unwrap_or("unknown".into());
        let interface_path = format!("/sys/class/net/{name}/operstate");

        let mut active = false;

        if let Ok(status) = fs::read_to_string(interface_path) {
            if status == "up\n" {
                active = true;
            }
        }

        Self {
            name,
            active,
            config,
        }
    }

    pub fn path(&self) -> PathBuf {
        Path::new(TUNNELS_PATH).join(format!("{}.conf", self.name))
    }

    /// Toggle actual interface using wireguard-tools.
    pub fn try_toggle(&mut self) -> Result<(), io::Error> {
        let config_path = self.path();

        let mut cmd = Command::new("wg-quick");

        cmd.args([
            if !self.active { "up" } else { "down" },
            config_path.to_str().unwrap(),
        ])
        .stderr(Stdio::piped());

        let mut proc = cmd.spawn()?;

        let code = proc.wait()?.code();

        if let Some(0) = code {
            Ok(())
        } else {
            let mut stderr = String::new();
            proc.stderr.unwrap().read_to_string(&mut stderr).unwrap();

            Err(io::Error::other(format!(
                "`wg-quick` exit code: {:?}. Error message:\n{}",
                code, stderr
            )))
        }
    }
}

#[derive(Debug)]
pub enum TunnelMsg {
    Toggle,
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
                connect_clicked[sender, index] => move |_| {
                    sender.output(Self::Output::Remove(index.clone())).unwrap();
                }
            },
        }
    }

    fn init_model(config: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self::new(config)
    }

    fn update_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
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
                _widgets.switch.set_state(self.active);
            }
        }
    }
}
