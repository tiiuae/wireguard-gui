/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use relm4::abstractions::Toaster;
use relm4::adw;
use relm4::gtk;
use relm4::gtk::pango;

pub enum ToastType {
    Error,
    Info,
}
#[derive(Default)]
pub struct ToastManager {
    toaster: Toaster,
}

impl ToastManager {
    fn create_label(&self, msg: &str, icon_text: &str, icon_color: &str) -> gtk::Label {
        gtk::Label::builder()
            .selectable(true)
            .wrap(true)
            .wrap_mode(pango::WrapMode::WordChar)
            .max_width_chars(70)
            .xalign(0.5)
            .halign(gtk::Align::Center)
            .justify(gtk::Justification::Center)
            .margin_start(12)
            .use_markup(true)
            .label(format!(
                "<span foreground='{}' font='20'><b>{}</b></span> <span foreground='white' font='13'><b>{}</b></span>",
                icon_color, icon_text, msg
            ))
            .build()
    }

    fn show_toast(&self, msg: &str, toast_type: ToastType) {
        let (icon_text, icon_color, timeout, priority) = match toast_type {
            ToastType::Error => ("✖", "red", 0, adw::ToastPriority::High),
            ToastType::Info => ("✓", "lime", 1, adw::ToastPriority::Normal),
        };

        let label = self.create_label(msg, icon_text, icon_color);

        let toast = adw::Toast::builder()
            .custom_title(&label)
            .timeout(timeout)
            .priority(priority)
            .build();

        self.toaster.add_toast(toast);
    }

    pub fn show_error(&self, msg: &str) {
        self.show_toast(msg, ToastType::Error);
    }

    pub fn show_info(&self, msg: &str) {
        self.show_toast(msg, ToastType::Info);
    }

    pub fn overlay_widget(&self) -> &adw::ToastOverlay {
        self.toaster.overlay_widget()
    }
}
