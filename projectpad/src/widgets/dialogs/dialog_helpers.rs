use diesel::prelude::*;
use gtk::prelude::*;

pub fn get_project_group_names(
    sql_conn: &diesel::SqliteConnection,
    project_id: i32,
) -> Vec<String> {
    use projectpadsql::schema::project_point_of_interest::dsl as ppoi;
    use projectpadsql::schema::server::dsl as srv;
    let server_group_names = srv::server
        .filter(
            srv::project_id
                .eq(project_id)
                .and(srv::group_name.is_not_null()),
        )
        .order(srv::group_name.asc())
        .select(srv::group_name)
        .load(sql_conn)
        .unwrap();
    let mut prj_poi_group_names = ppoi::project_point_of_interest
        .filter(
            ppoi::project_id
                .eq(project_id)
                .and(ppoi::group_name.is_not_null()),
        )
        .order(ppoi::group_name.asc())
        .select(ppoi::group_name)
        .load(sql_conn)
        .unwrap();
    let mut project_group_names = server_group_names;
    project_group_names.append(&mut prj_poi_group_names);
    let mut project_group_names_no_options: Vec<_> = project_group_names
        .into_iter()
        .map(|n: Option<String>| n.unwrap())
        .collect();
    project_group_names_no_options.sort();
    project_group_names_no_options.dedup();
    project_group_names_no_options
}

pub fn get_server_group_names(sql_conn: &diesel::SqliteConnection, server_id: i32) -> Vec<String> {
    use projectpadsql::schema::server_database::dsl as db;
    use projectpadsql::schema::server_extra_user_account::dsl as usr;
    use projectpadsql::schema::server_note::dsl as not;
    use projectpadsql::schema::server_point_of_interest::dsl as poi;
    use projectpadsql::schema::server_website::dsl as www;
    let server_poi_group_names = poi::server_point_of_interest
        .filter(
            poi::server_id
                .eq(server_id)
                .and(poi::group_name.is_not_null()),
        )
        .order(poi::group_name.asc())
        .select(poi::group_name)
        .load(sql_conn)
        .unwrap();
    let mut server_www_group_names = www::server_website
        .filter(
            www::server_id
                .eq(server_id)
                .and(www::group_name.is_not_null()),
        )
        .order(www::group_name.asc())
        .select(www::group_name)
        .load(sql_conn)
        .unwrap();
    let mut server_db_group_names = db::server_database
        .filter(
            db::server_id
                .eq(server_id)
                .and(db::group_name.is_not_null()),
        )
        .order(db::group_name.asc())
        .select(db::group_name)
        .load(sql_conn)
        .unwrap();
    let mut server_usr_group_names = usr::server_extra_user_account
        .filter(
            usr::server_id
                .eq(server_id)
                .and(usr::group_name.is_not_null()),
        )
        .order(usr::group_name.asc())
        .select(usr::group_name)
        .load(sql_conn)
        .unwrap();
    let mut server_notes_group_names = not::server_note
        .filter(
            not::server_id
                .eq(server_id)
                .and(not::group_name.is_not_null()),
        )
        .order(not::group_name.asc())
        .select(not::group_name)
        .load(sql_conn)
        .unwrap();
    let mut server_group_names = server_poi_group_names;
    server_group_names.append(&mut server_www_group_names);
    server_group_names.append(&mut server_db_group_names);
    server_group_names.append(&mut server_usr_group_names);
    server_group_names.append(&mut server_notes_group_names);
    let mut server_group_names_no_options: Vec<_> = server_group_names
        .into_iter()
        .map(|n: Option<String>| n.unwrap())
        .collect();
    server_group_names_no_options.sort();
    server_group_names_no_options.dedup();
    server_group_names_no_options
}

pub fn init_group_control(groups_store: &gtk::ListStore, group: &gtk::ComboBoxText) {
    let completion = gtk::EntryCompletion::new();
    completion.set_model(Some(groups_store));
    completion.set_text_column(0);
    group
        .get_child()
        .unwrap()
        .dynamic_cast::<gtk::Entry>()
        .unwrap()
        .set_completion(Some(&completion));
}

pub fn fill_groups(
    groups_store: &gtk::ListStore,
    group_widget: &gtk::ComboBoxText,
    groups: &[String],
    cur_group_name: &Option<String>,
) {
    for group in groups {
        let iter = groups_store.append();
        groups_store.set_value(&iter, 0, &glib::Value::from(&group));
        group_widget.append_text(&group);
    }

    if let Some(t) = cur_group_name.as_deref() {
        group_widget
            .get_child()
            .unwrap()
            .dynamic_cast::<gtk::Entry>()
            .unwrap()
            .set_text(t);
    }
}