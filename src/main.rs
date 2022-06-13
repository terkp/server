use rocket_dyn_templates::Template;

#[macro_use]
extern crate rocket;

mod group;
mod questions;
mod server_data;
mod display;

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(server_data::ServerData::default())
        .mount("/groups/", routes![group::new_group, group::get_all_groups])
        .mount(
            "/questions/",
            routes![
                questions::load_questions,
                questions::current_question,
                questions::next_question
            ],
        )
        .mount("/display", routes![
            display::show_display
        ])
        .attach(Template::fairing())
}
