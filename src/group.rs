use std::collections::HashMap;
use std::sync::atomic::Ordering;

use crate::server_data::{
    normalize_group_name, send_event, validate_group_name, Answer, GroupData, ServerData,
    UpdateEvent,
};
use rocket::http::Status;
use rocket::{serde::json::Json, State};
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize)]
pub struct NewGroupResponse {
    /// The canonical group name. The client stores this, not what was typed.
    name: String,
    /// False if a group with this name already existed (the client joins it).
    created: bool,
}

/// Creates a group, or joins the existing group of that name.
/// An unusable name is a 400 whose body is a message for the player.
#[post("/new", format = "text/plain", data = "<group>")]
pub async fn new_group(
    server_data: &State<ServerData>,
    group: String,
) -> Result<Json<NewGroupResponse>, (Status, String)> {
    let name = validate_group_name(&group).map_err(|e| (Status::BadRequest, e))?;
    let created = server_data.insert_group(name).is_ok();
    debug!("{:?}", server_data.groups);
    if created {
        //send group to anzeige and admin
        send_event(server_data, UpdateEvent::UpdateGroups);
    }
    Ok(Json(NewGroupResponse {
        name: name.to_owned(),
        created,
    }))
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

#[derive(Clone, Serialize, Deserialize)]
pub struct ScoreData {
    #[serde(alias = "name")]
    group_name: String,
    score: f64,
}

#[post("/set_score", format = "application/json", data = "<point_data>")]
pub async fn set_score(
    server_data: &State<ServerData>,
    point_data: Json<ScoreData>,
) -> (Status, String) {
    let group_name = normalize_group_name(&point_data.group_name);
    if !point_data.score.is_finite() {
        return (Status::BadRequest, "score is not a finite number".to_owned());
    }
    if let Err(e) = server_data.set_group_points(group_name, point_data.score) {
        return (Status::UnprocessableEntity, e.to_string());
    };
    send_event(server_data, UpdateEvent::UpdateGroups);
    (
        Status::Ok,
        format!(
            "set points for group '{}' to '{}'",
            group_name, point_data.score
        ),
    )
}

#[post("/add_score", format = "application/json", data = "<point_data>")]
pub async fn add_score(
    server_data: &State<ServerData>,
    point_data: Json<ScoreData>,
) -> (Status, String) {
    let group_name = normalize_group_name(&point_data.group_name);
    if !point_data.score.is_finite() {
        return (Status::BadRequest, "score is not a finite number".to_owned());
    }
    if let Err(e) = server_data.add_group_points(group_name, point_data.score) {
        return (Status::UnprocessableEntity, e.to_string());
    };
    send_event(server_data, UpdateEvent::UpdateGroups);
    (
        Status::Ok,
        format!(
            "added '{}' points to group '{}'",
            point_data.score, group_name
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
    /// The question version of the page the answer was given on. If present,
    /// the answer is only accepted while that is still the current question.
    #[serde(default)]
    question_version: Option<usize>,
}

/// Stores a group's answer to the current question.
///
/// - 400: the answer cannot be parsed or does not fit the current question
/// - 403: answers are no longer accepted (they were revealed)
/// - 409: the page the answer came from shows a question that is no longer current
/// - 410: the group does not exist (any more); the player has to log in again
#[post("/set_answer", format = "application/json", data = "<answer_data>")]
pub async fn set_answer(
    server_data: &State<ServerData>,
    answer_data: Json<AnswerData>,
) -> (Status, String) {
    let answer_data = answer_data.into_inner();
    let group_name = normalize_group_name(&answer_data.group_name);
    let answer =
        match Answer::try_parse_answer(&answer_data.answer_type, &answer_data.answer_string) {
            Ok(v) => v,
            Err(s) => return (Status::BadRequest, s),
        };

    {
        // The lock is held across the checks and the store, so a question
        // change or a reveal cannot happen in between.
        let questions = server_data.questions.lock().await;

        let current_version = server_data.question_version.load(Ordering::SeqCst);
        if answer_data
            .question_version
            .is_some_and(|version| version != current_version)
        {
            return (
                Status::Conflict,
                "the question has changed in the meantime".to_owned(),
            );
        }
        let current_question_idx = server_data.current_question.load(Ordering::SeqCst);
        let Some(question) = questions.get(current_question_idx) else {
            return (Status::Conflict, "there is no current question".to_owned());
        };
        if !answer.matches_question(question) {
            return (
                Status::BadRequest,
                "the answer does not fit the type of the current question".to_owned(),
            );
        }
        if server_data.block_answer.load(Ordering::SeqCst) {
            return (
                Status::Forbidden,
                "answers are no longer accepted for this question".to_owned(),
            );
        }
        if let Err(e) = server_data.set_group_answer(group_name, answer) {
            return (Status::Gone, e.to_string());
        }
    }
    send_event(server_data, UpdateEvent::UpdateGroups);
    (
        Status::Ok,
        format!("Updated group \"{group_name}\"'s answer"),
    )
}

#[post("/delete", format = "text/plain", data = "<group>")]
pub async fn del_group(server_data: &State<ServerData>, group: String) -> Status {
    server_data.delete_group(normalize_group_name(&group));
    debug!("{:?}", server_data.groups);
    //send group to anzeige and admin
    send_event(server_data, UpdateEvent::UpdateGroups);
    Status::Ok
}
