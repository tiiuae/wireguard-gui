// use gtk::prelude::*;
use relm4::prelude::*;

use crate::config::WireguardConfig;

pub struct OverviewModel {
    config: WireguardConfig,
}

#[derive(Debug)]
pub enum OverviewInput {
    CollectTunnel,
    ShowConfig(WireguardConfig),
}

#[derive(Debug)]
pub enum OverviewOutput {
    GenerateKeypair,
    SaveConfig(WireguardConfig),
}

#[relm4::component(pub)]
impl SimpleComponent for OverviewModel {
    type Init = WireguardConfig;
    type Input = OverviewInput;
    type Output = OverviewOutput;

    view! {
        gtk::Box {
            gtk::Frame::new(Some("hi")) {

            }
        }
    }

    fn init(
        config: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            config
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            Self::Input::CollectTunnel => todo!(),
            Self::Input::ShowConfig(config) => {
                self.config = dbg!(config);
            },
        }
    }
}
