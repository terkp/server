use std::sync::Mutex;
use rocket::{State, http::Status};
use crate::server_data::{ServerData, Question};

#[post("/load", format = "text/plain", data = "<questions>")]
pub fn load_questions(server_data: &State<Mutex<ServerData>>, questions: String) -> Status {
    let mut question_vec = Vec::new();

    for line in questions.lines() {
        if line.trim().is_empty() {
            continue
        }
        let question = match line.parse::<Question>() {
            Ok(q) => q,
            _ => return Status::BadRequest
        };
        question_vec.push(question);
    }

    let mut lock = server_data.lock().unwrap();
    lock.questions = question_vec;
    println!("{:?}", lock.questions);
    drop(lock);

    Status::Ok
}
