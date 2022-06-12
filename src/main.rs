use std::sync::Mutex;

#[macro_use]
extern crate rocket;

mod group;
mod server_data;
mod questions;

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(Mutex::new(server_data::ServerData::default()))
        .mount("/groups/", routes![
            group::new_group,
            group::get_all_groups
        ])
        .mount("/questions/", routes![
            questions::load_questions
        ])
}
