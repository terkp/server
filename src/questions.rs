use crate::server_data::{Question, ServerData, send_event, UpdateEvent};
use rocket::{http::Status, response::Redirect, serde::json::Json, State};
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
    println!("{:?}", server_data.questions);
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

#[get("/next")]
pub async fn next_question(server_data: &State<ServerData>) -> Redirect {
    server_data.current_question.fetch_add(1, Ordering::Relaxed);
    server_data.delete_group_answer().await;
    send_event(server_data, UpdateEvent::UpdateQuestions).await;
    Redirect::to(uri!("/questions", current_question()))
}

#[get("/results")]
pub async fn results(server_data: &State<ServerData>) -> Status {
    send_event(server_data, UpdateEvent::ShowSolution).await;
    server_data.results().await;
    Status::Ok
}
#[get("/show_solution")]
pub async fn show_solution(server_data: &State<ServerData>) -> Status {
    send_event(server_data, UpdateEvent::ShowSolution).await;
    Status::Ok
}

#[get("/show_answers")]
pub async fn show_answers(server_data: &State<ServerData>) -> Status {
    send_event(server_data, UpdateEvent::ShowAnswers).await;
    Status::Ok
}

#[get("/show_points")]
pub async fn show_points(server_data: &State<ServerData>) -> Status {
    send_event(server_data, UpdateEvent::ShowPoints).await;
    Status::Ok
}
