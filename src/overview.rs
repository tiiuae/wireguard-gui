// use gtk::prelude::*;
use relm4::{prelude::*, gtk::prelude::*};
use relm4::factory::{DynamicIndex, FactoryVecDeque};

use crate::config::*;
use crate::peer::*;

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
pub enum OverviewInput {
    CollectTunnel,
    ShowConfig(Box<WireguardConfig>),
    RemovePeer(DynamicIndex),
    AddPeer,
}

#[derive(Debug)]
pub enum OverviewOutput {
    GenerateKeypair,
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
            interface: config.interface, peers
        };

        model.replace_peers(config.peers);

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            Self::Input::CollectTunnel => todo!(),
            Self::Input::ShowConfig(config) => {
                let WireguardConfig { interface, peers } = *config;
                self.interface = interface;
                self.replace_peers(peers);
            },
            Self::Input::RemovePeer(idx) => {
                let mut peers = self.peers.guard();
                peers.remove(idx.current_index());
            },
            Self::Input::AddPeer => {
                let mut peers = self.peers.guard();
                peers.push_back(Peer::default());
            }
        }
    }
}
