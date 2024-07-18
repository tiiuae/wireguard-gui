use gtk::prelude::*;
use relm4::*;

pub mod log;
use log::*;

pub mod tunnels;
use tunnels::*;

pub mod header_bar;
use header_bar::*;

#[derive(Debug)]
enum AppMode {
    Tunnels,
    Logs,
}

#[derive(Debug)]
enum AppMsg {
    SetMode(AppMode),
}

struct AppModel {
    mode: AppMode,
    header: Controller<HeaderModel>,
    tunnels: Controller<TunnelsModel>,
    logs: Controller<LogsModel>,
}

#[relm4::component]
impl SimpleComponent for AppModel {
    type Init = AppMode;
    type Input = AppMsg;
    type Output = ();

    view! {
        main_window = gtk::Window {
            set_default_width: 500,
            set_default_height: 250,
            set_titlebar: Some(model.header.widget()),
            set_halign: gtk::Align::Fill,
            set_valign: gtk::Align::Fill,
            set_hexpand: true,
            set_vexpand: true,

            #[wrap(Some)]
            set_child = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                // set_halign: gtk::Align::Fill,
                set_hexpand: true,
                set_vexpand: true,
                match &model.mode {
                    AppMode::Tunnels => model.tunnels.widget().clone(),
                    AppMode::Logs => model.logs.widget().clone(),
                }
            }
        }
    }

    fn init(
        params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let header: Controller<HeaderModel> =
            HeaderModel::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    HeaderOutput::Tunnels => AppMsg::SetMode(AppMode::Tunnels),
                    HeaderOutput::Logs => AppMsg::SetMode(AppMode::Logs),
                });

        let logs = LogsModel::builder().launch(()).detach();

        let tunnels = TunnelsModel::builder()
            .launch(())
            .forward(logs.sender(), |msg| {
                println!("Forwarding string {:#?}", msg);
                match msg {
                    TunnelsOutput::LogEntry(s) => LogsInput::LogEntry(s),
                }
            });

        let model = AppModel {
            mode: params,
            header,
            tunnels,
            logs,
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AppMsg::SetMode(mode) => {
                self.mode = mode;
            }
        }
    }
}

fn main() {
    let relm = RelmApp::new("ewlm4.test.components");
    relm.run::<AppModel>(AppMode::Tunnels);
}
