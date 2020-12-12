use super::export;
use super::import_export_dtos::*;
use crate::sql_util::insert_row;
use diesel::dsl::count;
use diesel::prelude::*;
use projectpadsql::models::EnvironmentType;
use projectpadsql::sqlite_is;
use std::{fs, process};

type ImportResult<T> = Result<T, Box<dyn std::error::Error>>;

fn to_boxed_stderr(err: (String, Option<String>)) -> Box<dyn std::error::Error> {
    (err.0 + " - " + err.1.as_deref().unwrap_or("")).into()
}

pub fn do_import(
    sql_conn: &diesel::SqliteConnection,
    fname: &str,
    password: &str,
) -> ImportResult<()> {
    use projectpadsql::schema::project::dsl as prj;
    let mut temp_folder = export::temp_folder()?;

    // extract the 7zip...
    let seven_z_cmd = export::seven_z_command()?;
    if seven_z_cmd.is_none() {
        return Err("Need the 7z or 7za command to be installed".into());
    }
    let status = process::Command::new(seven_z_cmd.unwrap())
        .current_dir(&temp_folder)
        .args(&["x", &format!("-p{}", password), fname])
        .status()?;
    if !status.success() {
        return Err(format!("7z extraction failed: {:?}", status.code()).into());
    }

    temp_folder.push("contents.yaml");
    let contents = fs::read_to_string(&temp_folder); // no ? on purpose
    temp_folder.pop();
    fs::remove_dir_all(temp_folder)?;

    // only now fail if reading the file failed, we want
    // to remove the temp folder no matter what.
    let contents = contents?;

    let decoded: ProjectImportExport = serde_yaml::from_str(&contents)?;
    if prj::project
        .filter(prj::name.eq(&decoded.project_name))
        .select(count(prj::id))
        .first::<i64>(sql_conn)
        .unwrap()
        >= 1
    {
        return Err("A project with this name already exists".into());
    }
    let changeset = (
        prj::name.eq(decoded.project_name),
        prj::has_dev.eq(decoded.development_environment.is_some()),
        prj::has_stage.eq(decoded.staging_environment.is_some()),
        prj::has_uat.eq(decoded.uat_environment.is_some()),
        prj::has_prod.eq(decoded.prod_environment.is_some()),
        // TODO load the icon from the import 7zip
        prj::icon.eq(Some(Vec::<u8>::new())),
    );
    let project_id = insert_row(
        sql_conn,
        diesel::insert_into(prj::project).values(changeset),
    )
    .map_err(to_boxed_stderr)?;
    let mut unprocessed_websites = vec![];

    if let Some(dev_env) = decoded.development_environment {
        unprocessed_websites.extend(import_project_env_first_pass(
            sql_conn,
            project_id,
            EnvironmentType::EnvDevelopment,
            &dev_env,
        )?);
    }
    if let Some(stg_env) = decoded.staging_environment {
        unprocessed_websites.extend(import_project_env_first_pass(
            sql_conn,
            project_id,
            EnvironmentType::EnvStage,
            &stg_env,
        )?);
    }
    if let Some(uat_env) = decoded.uat_environment {
        unprocessed_websites.extend(import_project_env_first_pass(
            sql_conn,
            project_id,
            EnvironmentType::EnvUat,
            &uat_env,
        )?);
    }
    if let Some(prod_env) = decoded.prod_environment {
        unprocessed_websites.extend(import_project_env_first_pass(
            sql_conn,
            project_id,
            EnvironmentType::EnvProd,
            &prod_env,
        )?);
    }
    for unprocessed_website in unprocessed_websites {
        import_server_website(sql_conn, &unprocessed_website)?;
    }
    Ok(())
}

struct UnprocessedWebsite {
    server_id: i32,
    group_name: Option<String>,
    website: ServerWebsiteImportExport,
}

/// in the first pass we don't do server links and
/// server websites. server links can link to other
/// servers and websites and link to server databases.
///
/// we want to import all the potential link targets
/// in the first pass so the links are resolved, if
/// at all possible, when we'll process the second pass.
fn import_project_env_first_pass(
    sql_conn: &diesel::SqliteConnection,
    project_id: i32,
    env: EnvironmentType,
    project_env: &ProjectEnvImportExport,
) -> ImportResult<Vec<UnprocessedWebsite>> {
    let mut unprocessed_websites =
        import_project_env_group_first_pass(sql_conn, project_id, &project_env.items, env, None)?;

    for (group, items) in &project_env.items_in_groups {
        unprocessed_websites.append(&mut import_project_env_group_first_pass(
            sql_conn,
            project_id,
            &items,
            env,
            Some(group),
        )?);
    }

    Ok(unprocessed_websites)
}

fn import_project_env_group_first_pass(
    sql_conn: &diesel::SqliteConnection,
    project_id: i32,
    items: &ProjectEnvGroupImportExport,
    env: EnvironmentType,
    group_name: Option<&str>,
) -> ImportResult<Vec<UnprocessedWebsite>> {
    for project_poi in &items.project_pois {
        import_project_poi(sql_conn, project_id, group_name, project_poi)?;
    }
    for project_note in &items.project_notes {
        import_project_note(sql_conn, project_id, group_name, env, project_note)?;
    }
    for server_link in &items.server_links {
        import_server_link(sql_conn, project_id, group_name, env, server_link)?;
    }

    let mut unprocessed_websites = vec![];
    for server in &items.servers {
        unprocessed_websites.append(&mut import_server(
            sql_conn, project_id, env, group_name, server,
        )?);
    }
    Ok(unprocessed_websites)
}

fn import_project_poi(
    sql_conn: &diesel::SqliteConnection,
    project_id: i32,
    group_name: Option<&str>,
    project_poi: &ProjectPoiImportExport,
) -> ImportResult<()> {
    use projectpadsql::schema::project_point_of_interest::dsl as prj_poi;
    if project_poi.shared_with_other_environments.is_some() {
        return Ok(());
    }
    let changeset = (
        prj_poi::desc.eq(&project_poi.desc),
        prj_poi::path.eq(&project_poi.path),
        prj_poi::text.eq(&project_poi.text),
        prj_poi::group_name.eq(group_name),
        prj_poi::interest_type.eq(project_poi.interest_type),
        prj_poi::project_id.eq(project_id),
    );
    insert_row(
        sql_conn,
        diesel::insert_into(prj_poi::project_point_of_interest).values(changeset),
    )
    .map_err(to_boxed_stderr)
    .map(|_| ())
}

fn import_project_note(
    sql_conn: &diesel::SqliteConnection,
    project_id: i32,
    group_name: Option<&str>,
    env: EnvironmentType,
    project_note: &ProjectNoteImportExport,
) -> ImportResult<()> {
    use projectpadsql::schema::project_note::dsl as prj_note;
    if let Some(shared_title) = project_note.shared_with_other_environments.as_ref() {
        // update the row to mark that it's active
        // also for this environment
        let note_id_to_update = prj_note::project_note
            .select(prj_note::id)
            .filter(
                prj_note::title
                    .eq(&shared_title)
                    .and(sqlite_is(prj_note::group_name, group_name))
                    .and(prj_note::project_id.eq(project_id)),
            )
            .first::<i32>(sql_conn)?;
        let what = prj_note::project_note.filter(prj_note::id.eq(note_id_to_update));

        match env {
            // dev is the first, normally we come here at the 2nd
            // environment the earlier => skip it
            EnvironmentType::EnvStage => diesel::update(what)
                .set(prj_note::has_stage.eq(true))
                .execute(sql_conn),
            EnvironmentType::EnvUat => diesel::update(what)
                .set(prj_note::has_uat.eq(true))
                .execute(sql_conn),
            EnvironmentType::EnvProd => diesel::update(what)
                .set(prj_note::has_prod.eq(true))
                .execute(sql_conn),
            _ => unreachable!(),
        }
        .map(|_| ())?;
        Ok(())
    } else {
        // this note was not imported yet, import it the first time
        let changeset = (
            prj_note::title.eq(&project_note.title),
            prj_note::contents.eq(&project_note.contents),
            prj_note::has_dev.eq(env == EnvironmentType::EnvDevelopment),
            prj_note::has_stage.eq(env == EnvironmentType::EnvStage),
            prj_note::has_uat.eq(env == EnvironmentType::EnvUat),
            prj_note::has_prod.eq(env == EnvironmentType::EnvProd),
            prj_note::group_name.eq(group_name),
            prj_note::project_id.eq(project_id),
        );
        insert_row(
            sql_conn,
            diesel::insert_into(prj_note::project_note).values(changeset),
        )
        .map_err(to_boxed_stderr)
        .map(|_| ())
    }
}

fn import_server_link(
    sql_conn: &diesel::SqliteConnection,
    project_id: i32,
    group_name: Option<&str>,
    env: EnvironmentType,
    server_link: &ServerLinkImportExport,
) -> ImportResult<()> {
    use projectpadsql::schema::server_link::dsl as srv_link;
    let linked_server_id = get_linked_server_id(sql_conn, &server_link.server)?;
    let changeset = (
        srv_link::desc.eq(&server_link.desc),
        srv_link::group_name.eq(group_name),
        srv_link::linked_server_id.eq(linked_server_id),
        srv_link::project_id.eq(project_id),
        srv_link::environment.eq(env),
    );
    insert_row(
        sql_conn,
        diesel::insert_into(srv_link::server_link).values(changeset),
    )
    .map_err(to_boxed_stderr)
    .map(|_| ())
}

fn import_server(
    sql_conn: &diesel::SqliteConnection,
    project_id: i32,
    env: EnvironmentType,
    group_name: Option<&str>,
    server: &ServerWithItemsImportExport,
) -> ImportResult<Vec<UnprocessedWebsite>> {
    use projectpadsql::schema::server::dsl as srv;
    let changeset = (
        srv::desc.eq(&server.server.0.desc),
        srv::is_retired.eq(server.server.0.is_retired),
        srv::ip.eq(&server.server.0.ip),
        srv::text.eq(&server.server.0.text),
        srv::group_name.eq(group_name),
        srv::username.eq(&server.server.0.username),
        srv::password.eq(&server.server.0.password),
        srv::auth_key.eq(server.server.0.auth_key.as_ref()), // TODO probably stored elsewhere
        srv::auth_key_filename.eq(server.server.0.auth_key_filename.as_ref()),
        srv::server_type.eq(server.server.0.server_type),
        srv::access_type.eq(server.server.0.access_type),
        srv::environment.eq(env),
        srv::project_id.eq(project_id),
    );
    let server_id = insert_row(sql_conn, diesel::insert_into(srv::server).values(changeset))
        .map_err(to_boxed_stderr)?;

    import_server_items(sql_conn, server_id, None, &server.items)?;
    for (group_name, items) in &server.items_in_groups {
        import_server_items(sql_conn, server_id, Some(group_name), items)?;
    }

    let mut unprocessed_websites: Vec<_> = server
        .items
        .server_websites
        .iter()
        .map(|w| UnprocessedWebsite {
            server_id,
            group_name: None,
            website: w.clone(),
        })
        .collect();
    unprocessed_websites.extend(server.items_in_groups.iter().flat_map(|(k, v)| {
        v.server_websites.iter().map(move |www| UnprocessedWebsite {
            server_id,
            group_name: Some(k.to_string()),
            website: www.clone(),
        })
    }));
    Ok(unprocessed_websites)
}

fn import_server_items(
    sql_conn: &diesel::SqliteConnection,
    server_id: i32,
    group_name: Option<&str>,
    items: &ServerGroupImportExport,
) -> ImportResult<()> {
    for db in &items.server_databases {
        use projectpadsql::schema::server_database::dsl as srv_db;
        let changeset = (
            srv_db::desc.eq(&db.0.desc),
            srv_db::name.eq(&db.0.name),
            srv_db::group_name.eq(group_name),
            srv_db::text.eq(&db.0.text),
            srv_db::username.eq(&db.0.username),
            srv_db::password.eq(&db.0.password),
            srv_db::server_id.eq(server_id),
        );
        insert_row(
            sql_conn,
            diesel::insert_into(srv_db::server_database).values(changeset),
        )
        .map_err(to_boxed_stderr)?;
    }
    for note in &items.server_notes {
        use projectpadsql::schema::server_note::dsl as srv_note;
        let changeset = (
            srv_note::title.eq(&note.title),
            srv_note::group_name.eq(group_name),
            srv_note::contents.eq(&note.contents),
            srv_note::server_id.eq(server_id),
        );
        insert_row(
            sql_conn,
            diesel::insert_into(srv_note::server_note).values(changeset),
        )
        .map_err(to_boxed_stderr)?;
    }
    for poi in &items.server_pois {
        use projectpadsql::schema::server_point_of_interest::dsl as srv_poi;
        let changeset = (
            srv_poi::desc.eq(&poi.desc),
            srv_poi::path.eq(&poi.path),
            srv_poi::text.eq(&poi.text),
            srv_poi::group_name.eq(group_name),
            srv_poi::interest_type.eq(poi.interest_type),
            srv_poi::run_on.eq(poi.run_on),
            srv_poi::server_id.eq(server_id),
        );
        insert_row(
            sql_conn,
            diesel::insert_into(srv_poi::server_point_of_interest).values(changeset),
        )
        .map_err(to_boxed_stderr)?;
    }
    for user in &items.server_extra_users {
        use projectpadsql::schema::server_extra_user_account::dsl as srv_usr;
        let changeset = (
            srv_usr::desc.eq(&user.desc),
            srv_usr::group_name.eq(group_name),
            srv_usr::username.eq(&user.username),
            srv_usr::password.eq(&user.password),
            srv_usr::auth_key.eq(&user.auth_key), // TODO stored elsewhere?
            srv_usr::auth_key_filename.eq(&user.auth_key_filename),
            srv_usr::server_id.eq(server_id),
        );
        insert_row(
            sql_conn,
            diesel::insert_into(srv_usr::server_extra_user_account).values(changeset),
        )
        .map_err(to_boxed_stderr)?;
    }
    // server websites are handled in the second pass
    Ok(())
}

fn import_server_website(
    sql_conn: &diesel::SqliteConnection,
    website_info: &UnprocessedWebsite,
) -> ImportResult<()> {
    use projectpadsql::schema::server_website::dsl as srv_www;
    let new_databaseid = website_info
        .website
        .server_database
        .as_ref()
        .and_then(|db_path| get_new_databaseid(sql_conn, db_path).ok());
    let changeset = (
        srv_www::desc.eq(&website_info.website.desc),
        srv_www::url.eq(&website_info.website.url),
        srv_www::text.eq(&website_info.website.text),
        srv_www::group_name.eq(website_info.group_name.as_ref()),
        srv_www::username.eq(&website_info.website.username),
        srv_www::password.eq(&website_info.website.password),
        srv_www::server_database_id.eq(new_databaseid),
        srv_www::server_id.eq(website_info.server_id),
    );
    insert_row(
        sql_conn,
        diesel::insert_into(srv_www::server_website).values(changeset),
    )
    .map_err(to_boxed_stderr)?;
    Ok(())
}

fn get_linked_server_id(
    sql_conn: &diesel::SqliteConnection,
    server_path: &ServerPath,
) -> ImportResult<i32> {
    use projectpadsql::schema::project::dsl as prj;
    use projectpadsql::schema::server::dsl as srv;
    if let Some(id) = server_path.server_id {
        return Ok(id);
    }
    // server_id is not present, so I know that server_desc is present.
    Ok(srv::server
        .inner_join(prj::project)
        .select(srv::id)
        .filter(
            prj::name
                .eq(&server_path.project_name)
                .and(srv::environment.eq(server_path.environment))
                .and(srv::desc.eq(server_path.server_desc.as_ref().unwrap())),
        )
        .first::<i32>(sql_conn)?)
}

fn get_new_databaseid(
    sql_conn: &diesel::SqliteConnection,
    db_path: &ServerDatabasePath,
) -> ImportResult<i32> {
    use projectpadsql::schema::server_database::dsl as srv_db;
    if let Some(db_id) = db_path.database_id {
        return Ok(db_id);
    }

    // since database_id is not defined, i know that database_desc is.

    // first find the server id
    let server_id: i32 = match db_path.server_id {
        Some(id) => id,
        None => {
            // no server id, must find the server using desc, environment and project name
            use projectpadsql::schema::project::dsl as prj;
            use projectpadsql::schema::server::dsl as srv;
            srv::server
                .inner_join(prj::project)
                .select(srv::id)
                .filter(
                    prj::name
                        .eq(&db_path.project_name)
                        .and(srv::environment.eq(db_path.environment))
                        // we know server_desc is present, because server_id is not.
                        .and(srv::desc.eq(db_path.server_desc.as_ref().unwrap())),
                )
                .first(sql_conn)
                .map_err(|e| e.to_string())?
        }
    };
    Ok(srv_db::server_database
        .select(srv_db::id)
        .filter(
            srv_db::desc
                .eq(db_path.database_desc.as_ref().unwrap())
                .and(srv_db::server_id.eq(server_id)),
        )
        .first(sql_conn)
        .map_err(|e| e.to_string())?)
}
