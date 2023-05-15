use std::collections::HashMap;

use crate::server_data::{Answer, GroupData, ServerData};
use rocket::{serde::json::Json, State};
use serde::Deserialize;
use serde::Serialize;

#[post("/new", format = "text/plain", data = "<group>")]
pub async fn new_group(server_data: &State<ServerData>, group: String) -> String {
    server_data.insert_group(&group).await;
    println!("{:?}", server_data.groups);
    format!("Group \"{group}\" created")
}

#[get("/get")]
pub async fn get_all_groups(server_data: &State<ServerData>) -> Json<HashMap<String, GroupData>> {
    Json(server_data.groups.lock().await.clone())
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScoreData {
    #[serde(alias = "name")]
    group_name: String,
    score: isize,
}

#[post("/set_points", format = "application/json", data = "<point_data>")]
pub async fn set_points(
    server_data: &State<ServerData>,
    point_data: Json<ScoreData>,
) -> Result<(), String> {
    server_data
        .set_group_points(point_data.group_name.clone(), point_data.score, true)
        .await;
    Ok(())
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnswerData {
    #[serde(alias = "name")]
    group_name: String,
    #[serde(alias = "type")]
    answer_type: String,
    #[serde(rename = "answer")]
    answer_string: String,
}

impl TryInto<Answer> for AnswerData {
    type Error = String;

    fn try_into(self) -> Result<Answer, Self::Error> {
        Answer::try_parse_answer(self.answer_type, self.answer_string)
    }
}

#[post("/set_answer", format = "application/json", data = "<answer_data>")]
pub async fn set_answer(
    server_data: &State<ServerData>,
    answer_data: Json<AnswerData>,
) -> Result<String, String> {
    let group_name = answer_data.group_name.clone();
    let answer = answer_data.0.try_into()?;
    server_data.set_group_answer(&group_name, answer).await;

    Ok(format!("Updated group \"{group_name}\"'s answer"))
}

//#[get("/get")]
//pub fn get_group_data(server_data: &State<Mutex<ServerData>>) -> Json<(String, isize, Option<Answer>)> {

//}
