use std::{io, fs};

use gtk::prelude::*;
use relm4::prelude::*;

use crate::config::*;

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
            if status == "up\n".to_owned() {
                active = true;
                println!("active");
            }
        }

        Self {
            name,
            active,
            config,
        }
    }

    /// Toggle actual interface using wireguard-tools.
    pub fn toggle(&mut self) -> Result<(), io::Error> {
        // TODO
        Ok(())
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

            gtk::CheckButton {
                connect_toggled => Self::Input::Toggle,
                set_active: self.active,
                set_label: Some(&self.name),
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

    fn update(&mut self, msg: Self::Input, _sender: relm4::FactorySender<Self>) {
        match msg {
            Self::Input::Toggle => {
                // TODO
                let _ = self.toggle();
                self.active = !self.active;
            },
        }
    }
}
