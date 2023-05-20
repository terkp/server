use std::sync::atomic::Ordering;

use crate::server_data::{Question, ServerData};
use log::debug;
use rocket::{
    State,
};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;

#[derive(Serialize)]
struct GroupScore {
    name: String,
    score: isize,
}

#[get("/score")]
pub async fn show_score(server_data: &State<ServerData>) -> Template {
    let questions = &server_data.groups.lock().await;

    let groups = questions.iter().map(|(name, data)| GroupScore {name: name.clone(), score: data.score}).collect::<Vec<_>>();
    Template::render("display/score", context! {
        groups
    })
}

#[get("/")]
pub async fn show_display(server_data: &State<ServerData>) -> Template {
    let questions = &server_data.questions.lock().await;
    if questions.is_empty() {
        return Template::render(
            "display/estimate",
            context! { question: "Keine Frage Gefunden", solution: 0.0 },
        );
    }
    let question = &questions[server_data.current_question.load(Ordering::Relaxed)];

    let letters = ['A', 'B', 'C', 'D'];

    debug!("{}", serde_json::to_string(&question).unwrap());

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
                letters
            },
        ),
        Question::Estimate { question, solution } => Template::render(
            "display/estimate",
            context! {
                question,
                solution,
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
                letters
            },
        ),
    }
}

