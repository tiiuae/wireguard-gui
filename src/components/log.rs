use gtk::prelude::*;
use relm4::*;

#[derive(Default)]
pub struct LogsModel {
    latest_log: String,
}

#[derive(Debug)]
pub enum LogsInput {
    /// Passthrough logs from executed commands to logs tab.
    LogEntry(String),
}

pub struct LogsWidgets {
    pub text_view: gtk::TextView,
}

impl SimpleComponent for LogsModel {
    type Input = LogsInput;
    type Output = ();
    // Window width
    type Init = i32;
    type Root = gtk::Box;
    type Widgets = LogsWidgets;

    fn init_root() -> Self::Root {
        gtk::Box::builder()
            .build()
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = LogsModel::default();

        let text_view = gtk::TextView::builder()
            .width_request(init)
            .height_request(init)
            // .editable(false)
            // .cursor_visible(false)
            .build();

        text_view.emit_insert_at_cursor("Hello, world!");

        root.append(&text_view);

        // let label = gtk::Label::new(Some(&format!("Counter: {}", model.counter)));
        // label.set_margin_all(5);

        // window.set_child(Some(&vbox));
        // vbox.set_margin_all(5);
        // vbox.append(&inc_button);
        // vbox.append(&dec_button);
        // vbox.append(&label);

        // inc_button.connect_clicked(clone!(@strong sender => move |_| {
        //     sender.input(AppInput::Increment);
        // }));

        // dec_button.connect_clicked(clone!(@strong sender => move |_| {
        //     sender.input(AppInput::Decrement);
        // }));

        let widgets = LogsWidgets { text_view };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            LogsInput::LogEntry(s) => {
                self.latest_log = s;
            }
        }
    }

    /// Update the view to represent the updated model.
    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        widgets.text_view.emit_insert_at_cursor(&self.latest_log)
    }
}
