use std::sync::atomic::Ordering;

use rocket::tokio::time::{self, Duration};
use rocket::{
    http::Status,
    response::stream::{Event, EventStream},
    State,
};
use rocket_dyn_templates::{context, Template};
use rand::Rng;
use crate::server_data::{Question, ServerData, EventBuffer};

#[get("/")]
pub async fn show_display(server_data: &State<ServerData>) -> Template {
    let questions = &server_data.questions.lock().await;
    if questions.is_empty() {
        return Template::render("display/normal", context! { question: "Keine Frage Gefunden", solution: 0.0 });
    }
    let question = &questions[server_data.current_question.load(Ordering::Relaxed)];

    println!("{}", serde_json::to_string(&question).unwrap());


    match question {
        Question::Normal {
            question,
            answers,
            solution,
        } => Template::render(
            "display/normal",
            context! {
                question,
                answers,
                solution,
            },
        ),
        Question::Estimate { question, solution } => Template::render(
            "display/estimate",
            context! {
                question,
                solution
            },
        ),
        Question::Sort {
            question,
            answers,
            solution,
        } => Template::render(
            "display/sort",
            context! {
                question,
                answers,
                solution,
            },
        ),
    }
}

#[get("/events")]
pub async fn events(server_data: &State<ServerData>) -> EventStream![Event + '_] {
    let display_buffer = &server_data.display_buffer;

    let buffers = &mut server_data.client_event_buffers.lock().await;

    let ran_num: u8 = rand::thread_rng().gen();
    let key = ran_num.to_string();/// Hier random string bauen
    server_data.client_event_buffers.lock().await.insert(key.clone(), EventBuffer::new());
    // neuer random string

    // EVENT!
    // Schreibe in jeden event buffer

    // 0 1 2 3 4 5
    // a b c d e f

    EventStream! {
        loop {
            yield server_data.client_event_buffers.lock().await[&key].pop().await;
        }
    }
}

#[get("/send_event/<text>")]
pub async fn send_event(server_data: &State<ServerData>, text: String) -> Status {
    let display_buffer = &server_data.display_buffer;
    display_buffer.push(Event::data(text)).unwrap();
    Status::Ok
}
