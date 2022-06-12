use std::sync::Mutex;

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate serde;

mod group;
mod server_data;

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(Mutex::new(server_data::ServerData::default()))
        .mount("/groups/", routes![
            group::new_group,
            group::get_all_groups
        ])
}
