use super::dialog_helpers;
use crate::sql_thread::SqlFunc;
use gtk::prelude::*;
use projectpadsql::models::ServerExtraUserAccount;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg)]
pub enum Msg {}

pub struct Model {
    relm: relm::Relm<ServerExtraUserAddEditDialog>,
    db_sender: mpsc::Sender<SqlFunc>,
    server_id: i32,

    description: String,
    group_name: Option<String>,
    username: String,
    password: String,
    // TODO auth key
}

#[widget]
impl Widget for ServerExtraUserAddEditDialog {
    fn init_view(&mut self) {
        dialog_helpers::style_grid(&self.grid);
    }

    fn model(
        relm: &relm::Relm<Self>,
        params: (mpsc::Sender<SqlFunc>, i32, Option<ServerExtraUserAccount>),
    ) -> Model {
        let (db_sender, server_id, server_db) = params;
        let sd = server_db.as_ref();
        Model {
            relm: relm.clone(),
            db_sender,
            server_id,
            description: sd.map(|d| d.desc.clone()).unwrap_or_else(|| "".to_string()),
            group_name: sd.and_then(|s| s.group_name.clone()),
            username: sd
                .map(|d| d.username.clone())
                .unwrap_or_else(|| "".to_string()),
            password: sd
                .map(|d| d.password.clone())
                .unwrap_or_else(|| "".to_string()),
        }
    }

    fn update(&mut self, event: Msg) {}

    view! {
        #[name="grid"]
        gtk::Grid {
            gtk::Label {
                text: "Description",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 0,
                },
            },
            #[name="desc_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.description,
                cell: {
                    left_attach: 1,
                    top_attach: 0,
                },
            },
            gtk::Label {
                text: "Group",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 2,
                },
            },
            #[name="group"]
            gtk::ComboBoxText({has_entry: true}) {
                hexpand: true,
                cell: {
                    left_attach: 1,
                    top_attach: 2,
                },
            },
            gtk::Label {
                text: "Username",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 4,
                },
            },
            #[name="username_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.username,
                cell: {
                    left_attach: 1,
                    top_attach: 4,
                },
            },
            gtk::Label {
                text: "Password",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 5,
                },
            },
            #[name="password_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.password,
                visibility: false,
                input_purpose: gtk::InputPurpose::Password,
                cell: {
                    left_attach: 1,
                    top_attach: 5,
                },
            },
        }
    }
}
