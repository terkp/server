use crate::server_data::{send_event, Answer, Question, QuestionState, ServerData, UpdateEvent};

use rocket::{http::Status, response::Redirect, serde::json::Json, State};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

#[post("/load", format = "text/plain", data = "<questions>")]
pub async fn load_questions(server_data: &State<ServerData>, questions: String) -> Status {
    let mut question_vec = Vec::new();

    for line in questions.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let question = match line.parse::<Question>() {
            Ok(q) => q,
            _ => return Status::BadRequest,
        };
        question_vec.push(question);
    }

    *server_data.questions.lock().await = question_vec;
    server_data.current_question.store(0, Ordering::Relaxed);
    server_data.block_answer.store(false, Ordering::SeqCst);
    println!("{:?}", server_data.questions);
    server_data.question_state.reset();
    send_event(server_data, UpdateEvent::UpdateQuestions).await;
    Status::Ok
}

#[get("/current")]
pub async fn current_question(
    server_data: &State<ServerData>,
) -> (Status, Json<Option<(usize, Question)>>) {
    if server_data.current_question.load(Ordering::Relaxed)
        >= server_data.questions.lock().await.len()
    {
        return (Status::NotAcceptable, Json(None));
    }

    let current_question_idx = server_data.current_question.load(Ordering::Relaxed);
    let question = server_data.questions.lock().await[current_question_idx].clone();

    (Status::Ok, Json(Some((current_question_idx, question))))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupQuestionState {
    answer: Option<Answer>,
    #[serde(rename = "questionState")]
    question_state: QuestionState,
}

#[post("/state", format = "text/plain", data = "<group_name>")]
pub async fn current_question_state(
    server_data: &State<ServerData>,
    group_name: Option<String>,
) -> Result<Json<GroupQuestionState>, (Status, &'static str)> {
    let current_question_idx = server_data.current_question.load(Ordering::Relaxed);
    if server_data
        .questions
        .lock()
        .await
        .get(current_question_idx)
        .is_none()
    {
        return Err((Status::NotFound, "no question available"));
    };
    let answer = if let Some(group_name) = group_name {
        server_data
            .groups
            .get(&group_name)
            .and_then(|entry| entry.answer.clone())
    } else {
        None
    };

    Ok(Json(GroupQuestionState {
        answer,
        question_state: server_data.question_state.clone(),
    }))
}

#[get("/next")]
pub async fn next_question(server_data: &State<ServerData>) -> Redirect {
    server_data.current_question.fetch_add(1, Ordering::Relaxed);
    server_data.block_answer.store(false, Ordering::SeqCst);
    server_data.clear_group_answers().await;
    server_data.question_state.reset();
    send_event(server_data, UpdateEvent::UpdateQuestions).await;
    Redirect::to(uri!("/questions", current_question()))
}

#[post("/set", format = "text/plain", data = "<question_num>")]
pub async fn set_question(server_data: &State<ServerData>, question_num: String) -> Status {
    match question_num.parse::<usize>() {
        Ok(value) => {
            server_data.current_question.store(value, Ordering::Relaxed);
            server_data.block_answer.store(false, Ordering::SeqCst);
            server_data.question_state.reset();
            server_data.clear_group_answers().await;
            send_event(server_data, UpdateEvent::UpdateQuestions).await;
            Status::Ok
        }
        Err(err) => {
            println!("Failed to parse usize: {}", err);
            Status::BadRequest
        }
    }
}

#[get("/results")]
pub async fn results(server_data: &State<ServerData>) -> Result<(), String> {
    if let Err(e) = server_data.results().await {
        return Err(e.to_string());
    }
    server_data.question_state.show_solution();
    send_event(server_data, UpdateEvent::UpdateGroups).await;
    send_event(server_data, UpdateEvent::ShowSolution).await;
    Ok(())
}
#[get("/show_solution")]
pub async fn show_solution(server_data: &State<ServerData>) -> Status {
    server_data.question_state.show_solution();
    send_event(server_data, UpdateEvent::ShowSolution).await;
    Status::Ok
}

#[get("/show_answers")]
pub async fn show_answers(server_data: &State<ServerData>) -> Status {
    server_data.question_state.show_answers();
    send_event(server_data, UpdateEvent::ShowAnswers).await;
    server_data.block_answer.store(true, Ordering::SeqCst);
    Status::Ok
}

#[get("/show_points")]
pub async fn show_points(server_data: &State<ServerData>) -> Status {
    send_event(server_data, UpdateEvent::ShowPoints).await;
    Status::Ok
}
