use relm4::{prelude::*, typed_view::list::RelmListItem};

use crate::config::*;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Tunnel {
    pub name: String,
    pub config: WireguardConfig,
    pub active: bool,
}

impl Tunnel {
    pub fn new(config: WireguardConfig) -> Self {
        let cfg = config.clone();
        Self {
            active: false,
            name: config.interface.name.unwrap_or("unknown".into()),
            config: cfg,
        }
    }

    /// Toggles activation of a tunnel.
    ///
    /// Returns either new state, or error string from the command output.
    pub fn toggle(&mut self) -> Result<bool, String> {
        if self.active {
            self.active = false;
            Ok(self.active)
        } else {
            // TODO: Run command to activate tunnel.
            todo!();
            self.active = true;
        }
    }
}

impl Drop for Tunnel {
    fn drop(&mut self) {
        // TODO: Disconnect if connected
    }
}

// TODO: Add activity indication
impl RelmListItem for Tunnel {
    type Root = gtk::Box;
    // For now let entry be only label with name.
    type Widgets = gtk::Label;

    fn setup(_item: &gtk::ListItem) -> (gtk::Box, Self::Widgets) {
        relm4::view! {
            my_box = gtk::Box {
                #[name = "label"]
                gtk::Label,
            }
        }

        let widgets = label;

        (my_box, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        let label = widgets;
        label.set_label(&self.name);
    }
}
