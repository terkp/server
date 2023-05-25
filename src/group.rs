use std::collections::HashMap;

use crate::server_data::{send_event, UpdateEvent};
use crate::server_data::{Answer, GroupData, ServerData};
use rocket::http::Status;
use rocket::{serde::json::Json, State};
use serde::Deserialize;
use serde::Serialize;

#[post("/new", format = "text/plain", data = "<group>")]
pub async fn new_group(server_data: &State<ServerData>, group: String) -> (Status, String) {
    if let Err(e) = server_data.insert_group(&group) {
        return (Status::UnprocessableEntity, e.to_string());
    };
    debug!("{:?}", server_data.groups);
    //send group to anzeige and admin
    send_event(server_data, UpdateEvent::UpdateGroups).await;
    //format!("Group \"{group}\" created")
    (Status::Ok, format!("successfully inserted group '{group}'"))
}

#[get("/get")]
pub async fn get_all_groups(server_data: &State<ServerData>) -> Json<HashMap<String, GroupData>> {
    Json(
        server_data
            .groups
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect(),
    )
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScoreData {
    #[serde(alias = "name")]
    group_name: String,
    score: isize,
}

#[post("/set_score", format = "application/json", data = "<point_data>")]
pub async fn set_score(
    server_data: &State<ServerData>,
    point_data: Json<ScoreData>,
) -> (Status, String) {
    if let Err(e) = server_data.set_group_points(point_data.group_name.clone(), point_data.score) {
        return (Status::UnprocessableEntity, e.to_string());
    };
    (
        Status::Ok,
        format!(
            "set points for group '{}' to '{}'",
            point_data.group_name, point_data.score
        ),
    )
}

#[post("/add_score", format = "application/json", data = "<point_data>")]
pub async fn add_score(
    server_data: &State<ServerData>,
    point_data: Json<ScoreData>,
) -> (Status, String) {
    if let Err(e) = server_data.add_group_points(point_data.group_name.clone(), point_data.score) {
        return (Status::UnprocessableEntity, e.to_string());
    };
    (
        Status::Ok,
        format!(
            "set points for group '{}' to '{}'",
            point_data.group_name, point_data.score
        ),
    )
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnswerData {
    #[serde(alias = "name")]
    group_name: String,
    #[serde(alias = "type")]
    answer_type: String,
    #[serde(alias = "answer")]
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
) -> (Status, String) {
    let group_name = answer_data.group_name.clone();
    let answer = match dbg!(answer_data.0.try_into()) {
        Ok(v) => v,
        Err(s) => return (Status::InternalServerError, s),
    };
    if let Err(e) = server_data.set_group_answer(&group_name, answer) {
        return (Status::UnprocessableEntity, e.to_string());
    }
    send_event(server_data, UpdateEvent::UpdateGroups).await;
    (
        Status::Ok,
        format!("Updated group \"{group_name}\"'s answer"),
    )
}
#[post("/delete", format = "text/plain", data = "<group>")]
pub async fn del_group(server_data: &State<ServerData>, group: String) -> Status {
    server_data.delete_group(&group);
    debug!("{:?}", server_data.groups);
    //send group to anzeige and admin
    send_event(server_data, UpdateEvent::UpdateGroups).await;
    //format!("Group \"{group}\" created")
    Status::Ok
}

//#[get("/get")]
//pub fn get_group_data(server_data: &State<Mutex<ServerData>>) -> Json<(String, isize, Option<Answer>)> {

//}
