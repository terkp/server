use std::sync::Mutex;

use crate::server_data::{Answer, ServerData};
use rocket::{
    serde::json::Json,
    State,
};

#[post("/new", format = "application/json", data = "<group>")]
pub fn new_group(server_data: &State<Mutex<ServerData>>, group: String) -> String {
    let mut lock = server_data.lock().unwrap();

    lock.insert_group(&group);
    println!("{:?}", lock.groups);
    drop(lock);

    // Irgendwas machen
    format!("Gruppe \"{group}\" wurde empfangen!")
}

type GroupResponse = Vec<(String, isize, Option<Answer>)>;

#[get("/get")]
pub fn get_all_groups(server_data: &State<Mutex<ServerData>>) -> Json<GroupResponse> {
    let lock = server_data.lock().unwrap();

    Json(
        lock.groups
            .iter()
            .map(|(group_name, group_data)| {
                (
                    group_name.clone(),
                    group_data.score,
                    group_data.answer.clone(),
                )
            })
            .collect(),
    )
}
