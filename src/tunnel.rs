use std::{fs, io, path::PathBuf, process::Command};
use std::time::{SystemTime, UNIX_EPOCH};

use gtk::prelude::*;
use relm4::prelude::*;
use tar::{Builder, EntryType, Header, HeaderMode};

use crate::{config::*, utils};

// TODO: Accept as parameter.
const ALLOWED_IP: &str = "0.0.0.0/0";

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

    /// Toggle actual interface using wireguard-tools.
    pub fn toggle(&mut self) -> Result<(), io::Error> {
        let dir = tempfile::tempdir()?;

        let config_path = dir.path().join(format!("{}.conf", self.name));

        fs::write(&config_path, write_config(&self.config))?;

        Command::new("wg-quick")
            .args([
                if self.active { "up" } else { "down" },
                config_path.to_str().unwrap(),
            ])
            .spawn()?
            .wait()?;

        Ok(())
    }

    pub fn generate_configs(
        &mut self,
        _allowed_ip: &str,
    ) -> Result<Vec<WireguardConfig>, io::Error> {
        let config = &mut self.config;
        let mut missing_fields: Vec<String> = vec![];
        let interface_required_fields = [
            ("interface address", &config.interface.address),
            ("interface listen port", &config.interface.listen_port),
        ];
        for (name, _) in interface_required_fields
            .into_iter()
            .filter(|x| x.1.is_none())
        {
            missing_fields.push(name.to_string());
        }

        for (i, peer) in config.peers.iter().enumerate() {
            if peer.allowed_ips.is_none() {
                missing_fields.push(format!("peer {} allowed IPs", i + 1));
            }
        }

        if !missing_fields.is_empty() {
            let lines = missing_fields
                .iter()
                .map(|x| format!("- {}\n", x))
                .collect::<Vec<String>>();
            return Err(io::Error::other(format!(
                "Missing fields:\n{}",
                lines.concat()
            )));
        }

        let mut res = vec![];
        let priv_key = match &config.interface.private_key {
            Some(key) => key.clone(),
            None => utils::generate_private_key()?,
        };
        let pub_key = utils::generate_public_key(priv_key)?;

        for (i, peer) in config.peers.iter_mut().enumerate() {
            if peer.public_key.is_some() {
                continue;
            }

            let peer_priv_key = utils::generate_private_key()?;
            let peer_pub_key = utils::generate_public_key(peer_priv_key.clone())?;
            peer.public_key = Some(peer_pub_key);

            let interface = Interface {
                name: Some(format!("peer-{}", i + 1)),
                address: peer.allowed_ips.clone(),
                listen_port: config.interface.listen_port.clone(),
                private_key: Some(peer_priv_key),
                ..Default::default()
            };

            let ipeer = Peer {
                public_key: Some(pub_key.clone()),
                allowed_ips: config.interface.address.clone(),
                // allowed_ips: Some(allowed_ip.into()),
                ..Default::default()
            };

            res.push(WireguardConfig {
                interface,
                peers: vec![ipeer],
            });
        }

        res.insert(0, config.clone());

        Ok(res)
    }

    pub fn write_configs_to_path(&mut self, path: PathBuf) -> io::Result<()> {
        let cfgs = self.generate_configs(ALLOWED_IP)?;

        let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        let file = fs::File::create(path)?;
        let mut ar = Builder::new(file);
        ar.mode(HeaderMode::Complete);
        let mut header = Header::new_gnu();

        for cfg in cfgs.iter() {
            let mut name = cfg.interface.name.clone().unwrap();
            name.push_str(".conf");
            let content = write_config(cfg);

            header.set_size(content.as_bytes().len().try_into().unwrap());
            header.set_entry_type(EntryType::Regular);
            header.set_mtime(time.as_secs());
            header.set_mode(0o755);
            header.set_cksum();
            ar.append_data(&mut header, name, content.as_bytes())?;
        }

        ar.finish()
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

    fn update(&mut self, msg: Self::Input, sender: relm4::FactorySender<Self>) {
        match msg {
            Self::Input::Toggle => match self.toggle() {
                Ok(_) => self.active = !self.active,
                Err(err) => sender
                    .output_sender()
                    .emit(Self::Output::Error(err.to_string())),
            },
        }
    }
}
