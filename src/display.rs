use std::sync::atomic::Ordering;

use crate::server_data::{Question, ServerData};
use log::debug;
use rocket::State;
use rocket_dyn_templates::{context, Template};
use serde::Serialize;

#[derive(Serialize)]
struct GroupScore {
    name: String,
    score: f64,
}

#[get("/leaderboard")]
pub async fn show_leaderboard(server_data: &State<ServerData>) -> Template {
    let groups = server_data
        .groups
        .iter()
        .map(|entry| GroupScore {
            name: entry.key().clone(),
            score: entry.score,
        })
        .collect::<Vec<_>>();
    Template::render(
        "display/score",
        context! {
            groups
        },
    )
}

#[get("/")]
pub async fn show_display(server_data: &State<ServerData>) -> Template {
    let questions = &server_data.questions.lock().await;
    if questions.is_empty() {
        return Template::render(
            "display/waiting",
            context! { question: "Keine Frage Gefunden", solution: 0.0 },
        );
    }
    let question = &questions[server_data.current_question.load(Ordering::Relaxed)];
    let image_path = question.display_picture_path();

    let letters = ['A', 'B', 'C', 'D', 'E', 'F'];

    debug!("{}", serde_json::to_string(&question).unwrap());

    match question {
        Question::Normal {
            question,
            answers,
            solution,
            ..
        } => Template::render(
            "display/normal",
            context! {
                question,
                answers,
                solution,
                letters,
                image_path
            },
        ),
        Question::Estimate {
            question,
            solution,
            ..
        } => Template::render(
            "display/estimate",
            context! {
                question,
                solution,
                image_path,
            },
        ),
        Question::Sort {
            question,
            answers,
            solution,
            ..
        } => Template::render(
            "display/sort",
            context! {
                question,
                answers,
                solution,
                letters,
                image_path
            },
        ),
        Question::Zuordnen {
            question,
            answers,
            solution,
            ..
        } => {
            let solution_letters = solution
                .iter()
                .enumerate()
                .filter_map(|(idx, is_correct)| {
                    if *is_correct {
                        Some(letters[idx].to_string())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            Template::render(
                "display/zuordnen",
                context! {
                    question,
                    answers,
                    solution_letters,
                    letters,
                    image_path
                },
            )
        }
    }
}
