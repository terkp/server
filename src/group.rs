use std::collections::HashMap;
use std::collections::hash_map::Entry;

use crate::server_data::{GroupData, ServerData, Answer};
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

#[post("/set_answer", format ="application/json", data = "<answer_data>")]
pub async fn set_answer(server_data: &State<ServerData>, answer_data: &str) -> Result<String, String> {
    let v: Value = match serde_json::from_str(answer_data){
        Ok(text) => text,
        _ => json!(null),
    };
    if v == json!(null){
        return Err("Error by reading!".to_string())
    };
    let answer_type_temp = v["type"].as_str();
    let answer_type = answer_type_temp.as_deref().unwrap();
    let answer: Option<Answer>;
    if answer_type == "normal" {
        let answer_0_temp = v["answer_0"].as_str();
        let answer_0 = answer_0_temp.as_deref().unwrap();
        answer = Some(Answer::Normal(answer_to_number(answer_0)));

    }
    else if answer_type == "schaetzen" {
        let answer_0_temp = v["answer_0"].as_str();
        let answer_0 = answer_0_temp.as_deref().unwrap();
        match answer_0.parse::<f64>() {
            Ok(n) => answer = Some(Answer::Estimate(n)),
            Err(e) =>  return Err("Error by reading answer!".to_string()),
          }
    }
    else if answer_type == "sortier" {
        let answer_0_temp = v["answer_0"].as_str();
        let answer_0 = answer_0_temp.as_deref().unwrap();
        answer = Some(Answer::Sort(convert_letters_to_numbers(answer_0)));

    }
    else {
        answer = None;
    }

    let name_temp = v["name"].as_str();
    let name = name_temp.as_deref().unwrap();
    let name_s = String::from(name);
    match answer {
        Some(v) => server_data.set_group_answer(name_s,v).await,
        None => panic!("Error by reading, no answer found"),
    }
    Ok("OK".to_string())
}
pub fn answer_to_number(input:&str) -> usize{
    match input {
        "a" => 0,
        "b" => 1,
        "c" => 2,
        "d" => 3,
        _ => panic!("Error by reading"),
    }
}
pub fn convert_letters_to_numbers(letters: &str) -> [usize; 4] {
    let mut numbers = [0; 4];

    for (i, letter) in letters.chars().enumerate() {
        match letter {
            'a' => numbers[i] = 0,
            'b' => numbers[i] = 1,
            'c' => numbers[i] = 2,
            'd' => numbers[i] = 3,
            _ => panic!("Invalid letter: {}", letter),
        }
    }

    numbers
}
//#[get("/get")]
//pub fn get_group_data(server_data: &State<Mutex<ServerData>>) -> Json<(String, isize, Option<Answer>)> {

//}
