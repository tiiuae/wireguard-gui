// use gtk::prelude::*;
use relm4::prelude::*;

use crate::tunnel::Tunnel;

pub struct OverviewModel {
    tunnel: Tunnel,
}

#[derive(Debug)]
pub enum OverviewInput {
    CollectTunnel,
    ShowTunnel(Tunnel),
}

#[derive(Debug)]
pub enum OverviewOutput {
    GenerateKeypair,
    SaveTunnel(Tunnel),
}

#[relm4::component(pub)]
impl SimpleComponent for OverviewModel {
    type Init = Tunnel;
    type Input = OverviewInput;
    type Output = OverviewOutput;

    view! {
        gtk::Box {

        }
    }

    fn init(
        tunnel: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            tunnel
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            Self::Input::CollectTunnel => todo!(),
            Self::Input::ShowTunnel(tunnel) => {
                self.tunnel = tunnel;
            },
        }
    }
}
