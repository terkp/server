use std::collections::HashMap;

use crate::server_data::{GroupData, ServerData};
use rocket::{serde::json::Json, State};

#[post("/new", format = "application/json", data = "<group>")]
pub async fn new_group(server_data: &State<ServerData>, group: String) -> String {
    server_data.insert_group(&group).await;
    println!("{:?}", server_data.groups);

    format!("Gruppe \"{group}\" wurde empfangen!")
}

#[get("/get")]
pub async fn get_all_groups(server_data: &State<ServerData>) -> Json<HashMap<String, GroupData>> {
    Json(server_data.groups.lock().await.clone())
}

//#[get("/get")]
//pub fn get_group_data(server_data: &State<Mutex<ServerData>>) -> Json<(String, isize, Option<Answer>)> {

//}
