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
    let mut groups = server_data
        .groups
        .iter()
        .map(|entry| GroupScore {
            name: entry.key().clone(),
            score: entry.score,
        })
        .collect::<Vec<_>>();
    // Highest score first, equal scores by name.
    groups.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.name.cmp(&b.name))
    });
    Template::render(
        "display/score",
        context! {
            groups
        },
    )
}

#[get("/")]
pub async fn show_display(server_data: &State<ServerData>) -> Template {
    // Index and version are read under the questions lock, so the page never
    // carries a version that is newer than its content.
    let questions = &server_data.questions.lock().await;
    let question_version = server_data.question_version.load(Ordering::SeqCst);
    let current_question_idx = server_data.current_question.load(Ordering::SeqCst);

    // No questions loaded, or the quiz is finished.
    let Some(question) = questions.get(current_question_idx) else {
        return Template::render(
            "display/waiting",
            context! { question: "Keine Frage Gefunden", solution: 0.0, question_version },
        );
    };
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
                image_path,
                question_version
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
                question_version
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
                image_path,
                question_version
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
                    image_path,
                    question_version
                },
            )
        }
    }
}
