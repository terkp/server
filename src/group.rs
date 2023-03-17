use std::collections::HashMap;
use std::collections::hash_map::Entry;

use crate::server_data::{GroupData, ServerData};
use rocket::{http::Status,serde::json::Json, State};
use serde_json::json;
use serde_json::{Value, Error};
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
#[post("/points_set", format ="application/json", data = "<name_number>")]
pub async fn set_points(server_data: &State<ServerData>, name_number: &str) -> Result<String, String> {
    let v: Value = match serde_json::from_str(name_number){
        Ok(text) => text,
        _ => json!(null),
    };
    if v == json!(null){
        return Err("Error by reading!".to_string())
    };
    let name_temp = v["name"].as_str();
    let name = name_temp.as_deref().unwrap();
    let number_temp = v["number"].as_str();
    let number = number_temp.as_deref().unwrap();
    let set_number_unchecked = number.parse::<i32>();
    
    let set_number = match set_number_unchecked {
        Ok(numb) => numb,
        _ => return Err("Not a number!".to_string()),
    };
    let set_number_i = set_number as isize;
    let name_s = String::from(name);
    server_data.set_group_points(name_s,set_number_i,true).await;
    Ok("OK".to_string())
}

//#[get("/get")]
//pub fn get_group_data(server_data: &State<Mutex<ServerData>>) -> Json<(String, isize, Option<Answer>)> {

//}
