use super::search_view::{Area, ProjectPadItem, SEARCH_RESULT_WIDGET_HEIGHT};
use crate::icons::*;
use gdk::prelude::GdkContextExt;
use gtk::prelude::*;
use projectpadsql::models::{
    EnvironmentType, Project, ProjectNote, ProjectPointOfInterest, Server, ServerDatabase,
    ServerExtraUserAccount, ServerLink, ServerNote, ServerPointOfInterest, ServerWebsite,
};
const LEFT_RIGHT_MARGIN: i32 = 150;
const ACTION_ICON_SIZE: i32 = 16;
const PROJECT_ICON_SIZE: i32 = 56;
const ACTION_ICON_OFFSET_FROM_RIGHT: f64 = 50.0;

fn draw_button(context: &cairo::Context, x: f64, y: f64, w: f64, h: f64) {
    let style_context = &gtk::StyleContext::new();
    let path = gtk::WidgetPath::new();
    path.append_type(glib::Type::Invalid);
    path.iter_set_object_name(-1, Some("button"));
    style_context.set_path(&path);
    style_context.add_class(&gtk::STYLE_CLASS_BUTTON);
    style_context.add_class("image-button");

    gtk::render_background(style_context, context, x, y, w, h);

    gtk::render_frame(style_context, context, x, y, w, h);
}

fn draw_box(
    hierarchy_offset: f64,
    style_context: &gtk::StyleContext,
    y: f64,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
) {
    let margin = style_context.get_margin(gtk::StateFlags::NORMAL);
    gtk::render_background(
        style_context,
        context,
        margin.left as f64 + hierarchy_offset,
        y + margin.top as f64,
        search_result_area.get_allocation().width as f64
            - margin.left as f64
            - margin.right as f64
            - hierarchy_offset * 2.0,
        SEARCH_RESULT_WIDGET_HEIGHT as f64 - margin.top as f64,
    );

    // https://github.com/GNOME/gtk/blob/ca71340c6bfa10092c756e5fdd5e41230e2981b5/gtk/theme/Adwaita/gtk-contained.css#L1599
    // use the system theme's frame class
    style_context.add_class(&gtk::STYLE_CLASS_FRAME);
    gtk::render_frame(
        style_context,
        context,
        margin.left as f64 + hierarchy_offset,
        y as f64 + margin.top as f64,
        search_result_area.get_allocation().width as f64
            - margin.left as f64
            - margin.right as f64
            - hierarchy_offset * 2.0,
        SEARCH_RESULT_WIDGET_HEIGHT as f64 - margin.top as f64,
    );
    style_context.remove_class(&gtk::STYLE_CLASS_BUTTON);
}

pub fn draw_child(
    style_context: &gtk::StyleContext,
    item: &ProjectPadItem,
    y: i32,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
    links: &mut Vec<(Area, String)>,
) {
    let extra_css_class = match item {
        ProjectPadItem::Server(_) => "search_view_parent",
        _ => "search_view_child",
    };
    style_context.add_class(extra_css_class);
    let padding = style_context.get_padding(gtk::StateFlags::NORMAL);
    match item {
        ProjectPadItem::Project(p) => draw_project(
            style_context,
            context,
            search_result_area,
            padding.left as f64 + LEFT_RIGHT_MARGIN as f64,
            y as f64,
            &p,
        ),
        ProjectPadItem::Server(s) => draw_server(
            style_context,
            context,
            &padding,
            LEFT_RIGHT_MARGIN as f64,
            search_result_area,
            padding.left as f64 + LEFT_RIGHT_MARGIN as f64,
            y as f64,
            &s,
        ),
        ProjectPadItem::ServerWebsite(w) => draw_server_website(
            style_context,
            context,
            search_result_area,
            padding.left as f64 + LEFT_RIGHT_MARGIN as f64,
            y as f64,
            &w,
            links,
        ),
        _ => {
            draw_box(
                LEFT_RIGHT_MARGIN as f64,
                style_context,
                y as f64,
                context,
                search_result_area,
            );
        }
    }
    style_context.remove_class(extra_css_class);
}

fn draw_project(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
    x: f64,
    y: f64,
    project: &Project,
) {
    // since the servers have 10px padding on top of them,
    // let's draw the projects at the bottom of their area
    // so, y+height-icon_size
    let padding = style_context.get_padding(gtk::StateFlags::NORMAL);
    let title_extents = draw_title(
        style_context,
        context,
        &padding,
        search_result_area,
        &project.name,
        x,
        y + SEARCH_RESULT_WIDGET_HEIGHT as f64 - PROJECT_ICON_SIZE as f64,
        Some(PROJECT_ICON_SIZE),
    );

    if let Some(icon) = &project.icon {
        if icon.len() > 0 {
            let translate_x = x + (title_extents.width / 1024) as f64 + padding.left as f64;
            let translate_y = y + padding.top as f64 + SEARCH_RESULT_WIDGET_HEIGHT as f64
                - PROJECT_ICON_SIZE as f64;
            context.translate(translate_x, translate_y);
            super::project_badge::ProjectBadge::draw_icon(context, PROJECT_ICON_SIZE, &icon);
            context.translate(-translate_x, -translate_y);
        }
    }
}

fn draw_server_website(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
    x: f64,
    y: f64,
    website: &ServerWebsite,
    links: &mut Vec<(Area, String)>,
) {
    let padding = style_context.get_padding(gtk::StateFlags::NORMAL);
    let margin = style_context.get_margin(gtk::StateFlags::NORMAL);
    draw_box(
        LEFT_RIGHT_MARGIN as f64,
        style_context,
        y,
        context,
        search_result_area,
    );
    draw_icon(
        style_context,
        context,
        &Icon::HTTP,
        x + padding.left as f64,
        y + margin.top as f64 + padding.top as f64,
    );
    let title_rect = draw_title(
        style_context,
        context,
        &padding,
        search_result_area,
        &website.desc,
        x + ACTION_ICON_SIZE as f64 + (padding.left / 2) as f64,
        y + margin.top as f64,
        Some(ACTION_ICON_SIZE),
    );
    draw_link(
        style_context,
        context,
        search_result_area,
        &website.url,
        x + padding.left as f64,
        y + margin.top as f64 + (title_rect.height / 1024) as f64 + padding.top as f64,
        links,
    );

    draw_action(
        style_context,
        context,
        &Icon::COG,
        search_result_area.get_allocation().width as f64
            - ACTION_ICON_OFFSET_FROM_RIGHT
            - LEFT_RIGHT_MARGIN as f64,
        y + padding.top as f64 + margin.top as f64,
    );
}

fn draw_server(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    padding: &gtk::Border,
    hierarchy_offset: f64,
    search_result_area: &gtk::DrawingArea,
    x: f64,
    y: f64,
    server: &Server,
) {
    let margin = style_context.get_margin(gtk::StateFlags::NORMAL);
    draw_box(
        hierarchy_offset,
        style_context,
        y,
        context,
        search_result_area,
    );
    let title_rect = draw_title(
        style_context,
        context,
        &padding,
        search_result_area,
        &server.desc,
        x,
        y + margin.top as f64,
        None,
    );
    draw_environment(
        style_context,
        context,
        search_result_area,
        x + padding.left as f64,
        y + (title_rect.height / 1024) as f64 + padding.top as f64 + margin.top as f64,
        &match server.environment {
            EnvironmentType::EnvUat => "uat",
            EnvironmentType::EnvProd => "prod",
            EnvironmentType::EnvStage => "stg",
            EnvironmentType::EnvDevelopment => "dev",
        },
    );
    draw_action(
        style_context,
        context,
        &Icon::COG,
        search_result_area.get_allocation().width as f64
            - ACTION_ICON_OFFSET_FROM_RIGHT
            - LEFT_RIGHT_MARGIN as f64,
        y + padding.top as f64 + margin.top as f64,
    );
}

fn draw_environment(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
    x: f64,
    y: f64,
    env_name: &str,
) {
    let label_classname = format!("environment_label_{}", env_name);
    style_context.add_class(&label_classname);
    let padding = style_context.get_padding(gtk::StateFlags::NORMAL);
    let pango_context = search_result_area
        .create_pango_context()
        .expect("failed getting pango context");
    let layout = pango::Layout::new(&pango_context);
    layout.set_text(&env_name.to_uppercase());
    let rect = layout.get_extents().1;
    let text_w = (rect.width / 1024) as f64;
    let text_h = (rect.height / 1024) as f64;

    gtk::render_background(
        style_context,
        context,
        x,
        y,
        text_w + padding.left as f64 + padding.right as f64,
        text_h + padding.top as f64 + padding.bottom as f64,
    );

    gtk::render_frame(
        style_context,
        context,
        x,
        y,
        text_w + padding.left as f64 + padding.right as f64,
        text_h + padding.top as f64 + padding.bottom as f64,
    );

    gtk::render_layout(
        style_context,
        context,
        x + padding.left as f64,
        y + padding.top as f64,
        &layout,
    );
    style_context.remove_class(&label_classname);
}

fn draw_title(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    padding: &gtk::Border,
    search_result_area: &gtk::DrawingArea,
    text: &str,
    x: f64,
    y: f64,
    height: Option<i32>,
) -> pango::Rectangle {
    style_context.add_class("search_result_item_title");
    let pango_context = search_result_area
        .create_pango_context()
        .expect("failed getting pango context");
    let layout = pango::Layout::new(&pango_context);
    layout.set_text(text);
    layout.set_ellipsize(pango::EllipsizeMode::End);
    layout.set_width(350 * 1024);
    let extra_y = if let Some(h) = height {
        let layout_height = layout.get_extents().1.height as f64 / 1024.0;
        (h as f64 - layout_height) / 2.0
    } else {
        0.0
    };
    gtk::render_layout(
        style_context,
        context,
        x + padding.left as f64,
        y + padding.top as f64 + extra_y,
        &layout,
    );
    style_context.remove_class("search_result_item_title");

    layout.get_extents().1
}

fn draw_link(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
    text: &str,
    x: f64,
    y: f64,
    links: &mut Vec<(Area, String)>,
) -> pango::Rectangle {
    style_context.add_class("search_result_item_link");
    let padding = style_context.get_padding(gtk::StateFlags::NORMAL);
    let pango_context = search_result_area
        .create_pango_context()
        .expect("failed getting pango context");
    let layout = pango::Layout::new(&pango_context);
    layout.set_text(text);
    layout.set_ellipsize(pango::EllipsizeMode::End);
    layout.set_width(350 * 1024);
    let left = x + padding.left as f64;
    let top = y + padding.top as f64;
    gtk::render_layout(style_context, context, left, top, &layout);

    let extents = layout.get_extents().1;

    links.push((
        Area::new(
            left as i32,
            top as i32,
            extents.width / 1024,
            extents.height / 1024,
        ),
        text.to_string(),
    ));

    style_context.remove_class("search_result_item_link");
    extents
}

fn draw_action(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    icon: &Icon,
    x: f64,
    y: f64,
) {
    style_context.add_class("search_result_action_btn");
    let padding = style_context.get_padding(gtk::StateFlags::NORMAL);
    draw_button(
        context,
        x,
        y,
        ACTION_ICON_SIZE as f64 + (padding.left + padding.right) as f64,
        ACTION_ICON_SIZE as f64 + (padding.top + padding.bottom) as f64,
    );
    style_context.remove_class("search_result_action_btn");
    draw_icon(
        style_context,
        context,
        icon,
        x + padding.left as f64,
        y + padding.top as f64,
    );
}

fn draw_icon(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    icon: &Icon,
    x: f64,
    y: f64,
) {
    // we know we use symbolic (single color) icons.
    // i want to paint them in the theme's foreground color
    // (important for dark themes).
    // the way that I found is to paint a mask.

    // 1. load the icon as a pixbuf...
    let pixbuf = gtk::IconTheme::get_default()
        .expect("get icon theme")
        .load_icon(
            icon.name(),
            ACTION_ICON_SIZE,
            gtk::IconLookupFlags::FORCE_SYMBOLIC,
        )
        .expect("load icon1")
        .expect("load icon2");

    // 2. create a cairo surface, paint the pixbuf on it...
    let surf =
        cairo::ImageSurface::create(cairo::Format::ARgb32, ACTION_ICON_SIZE, ACTION_ICON_SIZE)
            .expect("ImageSurface");
    let surf_context = cairo::Context::new(&surf);
    surf_context.set_source_pixbuf(&pixbuf, 0.0, 0.0);
    surf_context.paint();

    // 3. set the foreground color of our context to the theme's fg color
    let fore_color = style_context.get_color(gtk::StateFlags::NORMAL);
    context.set_source_rgba(
        fore_color.red,
        fore_color.green,
        fore_color.blue,
        fore_color.alpha,
    );

    // 4. use the surface we created with the icon as a mask
    // (the alpha channel of the surface is mixed with the context
    // color to paint)
    context.mask_surface(&surf, x, y);
}