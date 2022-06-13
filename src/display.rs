use std::sync::atomic::Ordering;

use rocket::{State, response::stream::{Event, EventStream}, http::Status};
use rocket_dyn_templates::{Template, context};
use rocket::tokio::time::{self, Duration};

use crate::{server_data::{ServerData, Question}};


#[get("/")]
pub async fn show_display(server_data: &State<ServerData>) -> Template {

    let question = &server_data.questions.lock().await[
        server_data.current_question.load(Ordering::Relaxed)
    ];

    println!("{}", serde_json::to_string(question).unwrap());

    match question {
        Question::Normal { question, answers, solution } => {
            Template::render("display/normal", context! {
                question,
                answers,
                solution,
            })
        },
        Question::Estimate { question, solution } => {
            Template::render("display/estimate", context! {
                question,
                solution
            })
        },
        Question::Sort { question, answers, solution } => {
            Template::render("display/sort", context! {
                question,
                answers,
                solution,
            })
        }
    }
}

#[get("/events")]
pub async fn events(server_data: &State<ServerData>) -> EventStream![Event + '_] {
    let display_buffer = &server_data.display_buffer;
    EventStream! {
        loop {
            yield display_buffer.pop().await;
        }
    }
}


#[get("/send_event/<text>")]
pub async fn send_event(server_data: &State<ServerData>, text: String) -> Status {
    let display_buffer = &server_data.display_buffer;
    display_buffer.push(Event::data(text)).unwrap();
    Status::Ok
}