use std::sync::atomic::Ordering;

use rocket::{State, response::stream::{Event, EventStream}};
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
pub fn events() -> EventStream![] {
    EventStream! {
        let mut interval = time::interval(Duration::from_secs(1));
        loop {
            yield Event::data("ping");
            interval.tick().await;
        }
    }
}