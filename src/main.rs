use std::path::{PathBuf, Path};

use rocket::fs::NamedFile;
use rocket_dyn_templates::Template;

#[macro_use]
extern crate rocket;

mod group;
mod questions;
mod server_data;
mod display;

#[get("/<file..>")]
async fn files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("frontend/").join(file)).await.ok()
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(server_data::ServerData::default())
        .mount("/", routes![files])
        .mount("/groups/", routes![group::new_group, group::get_all_groups, group::set_points])
        .mount(
            "/questions/",
            routes![
                questions::load_questions,
                questions::current_question,
                questions::next_question
            ],
        )
        .mount("/display", routes![
            display::show_display,
            display::events,
            display::send_event
        ])
        .attach(Template::fairing())
}
