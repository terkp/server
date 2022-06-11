#[macro_use] extern crate rocket;

mod group;

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![
        group::new_group
    ])
}
