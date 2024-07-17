use gtk::prelude::*;
use relm4::*;

#[derive(Default)]
pub struct LogsModel;

#[derive(Debug)]
pub enum LogsInput {
    /// Passthrough logs from executed commands to logs tab.
    LogEntry(String),
}

pub struct LogsWidgets {
    pub text_view: gtk::TextView,
}

impl Component for LogsModel {
    type CommandOutput = ();
    type Input = LogsInput;
    type Output = ();
    type Init = ();
    type Root = gtk::Box;
    type Widgets = LogsWidgets;

    fn init_root() -> Self::Root {
        gtk::Box::builder().build()
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = LogsModel::default();

        let text_view = gtk::TextView::builder()
            .hexpand(true)
            .vexpand(true)
            .editable(false)
            .cursor_visible(false)
            .build();

        root.append(&text_view);

        let widgets = LogsWidgets { text_view };

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            LogsInput::LogEntry(s) => {
                widgets.text_view.set_editable(true);
                widgets.text_view.emit_insert_at_cursor(&s);
                widgets.text_view.set_editable(false);
            }
        }
    }
}
