use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::{Project, Server};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::collections::HashSet;
use std::sync::mpsc;

pub struct SearchResult {
    pub projects: Vec<Project>,
}

#[derive(Msg)]
pub enum Msg {
    FilterChanged(Option<String>),
    GotSearchResult(SearchResult),
}

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    filter: Option<String>,
    sender: relm::Sender<SearchResult>,
}

#[widget]
impl Widget for SearchView {
    fn init_view(&mut self) {}

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        let stream = relm.stream().clone();
        let (channel, sender) = relm::Channel::new(move |search_r: SearchResult| {
            stream.emit(Msg::GotSearchResult(search_r));
        });
        Model {
            filter: None,
            db_sender,
            sender,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::FilterChanged(filter) => {
                self.model.filter = filter;
                self.fetch_search_results();
            }
            Msg::GotSearchResult(search_result) => {
                println!(
                    "got results {:?}!",
                    search_result
                        .projects
                        .iter()
                        .map(|p| &p.name)
                        .collect::<Vec<_>>()
                );
            }
        }
    }

    fn fetch_search_results(&self) {
        println!("refresh");
        match &self.model.filter {
            None => self
                .model
                .sender
                .send(SearchResult { projects: vec![] })
                .unwrap(),
            Some(filter) => {
                let s = self.model.sender.clone();
                let f = format!("%{}%", filter.replace('%', "\\%"));
                self.model
                    .db_sender
                    .send(SqlFunc::new(move |sql_conn| {
                        let servers = Self::load_servers(sql_conn, &f);
                        let prjs = Self::load_projects(sql_conn, &f);
                        let mut all_project_ids =
                            servers.iter().map(|s| s.project_id).collect::<HashSet<_>>();
                        all_project_ids.extend(prjs.iter().map(|p| p.id));
                        let all_projects = Self::load_projects_by_id(sql_conn, &all_project_ids);
                        s.send(SearchResult {
                            projects: all_projects,
                        })
                        .unwrap();
                    }))
                    .unwrap();
            }
        }
    }

    fn load_projects(db_conn: &SqliteConnection, filter: &str) -> Vec<Project> {
        use projectpadsql::schema::project::dsl::*;
        project
            .filter(name.like(filter).escape('\\'))
            .load::<Project>(db_conn)
            .unwrap()
    }

    fn load_projects_by_id(db_conn: &SqliteConnection, ids: &HashSet<i32>) -> Vec<Project> {
        use projectpadsql::schema::project::dsl::*;
        project
            .filter(id.eq_any(ids))
            .load::<Project>(db_conn)
            .unwrap()
    }

    fn load_servers(db_conn: &SqliteConnection, filter: &str) -> Vec<Server> {
        use projectpadsql::schema::server::dsl::*;
        server
            .filter(desc.like(filter).escape('\\'))
            .load::<Server>(db_conn)
            .unwrap()
    }

    view! {
        gtk::ScrolledWindow {
            #[name="search_result_box"]
            gtk::Box {
                orientation: gtk::Orientation::Vertical,
                spacing: 10,
            }
        }
    }
}
