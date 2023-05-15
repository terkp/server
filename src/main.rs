use std::{path::{Path, PathBuf}, sync::atomic::Ordering};

use rand::{distributions::Alphanumeric, Rng};
use rocket::{fs::NamedFile, State, response::stream::{Event, EventStream}};
use rocket_dyn_templates::{Template, context};
use simplelog::{TermLogger, ConfigBuilder};

use crate::server_data::{ServerData, EVENT_BUFFER_KEY_LENGTH, EventBuffer, Question};


#[macro_use]
extern crate rocket;

mod client;
mod display;
mod group;
mod questions;
mod server_data;

#[get("/<file..>")]
async fn files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("frontend/").join(file))
        .await
        .ok()
}

fn setup_logger() {
    let cfg = ConfigBuilder::new()
        .add_filter_allow_str("server")
        .add_filter_allow_str("rocket")
        .build();

    TermLogger::init(
        log::LevelFilter::Debug,
        cfg,
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Always,
    )
    .unwrap();
}

#[get("/events")]
pub async fn events(server_data: &State<ServerData>) -> EventStream![Event + '_] {
    let key = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(EVENT_BUFFER_KEY_LENGTH)
        .map(char::from)
        .collect::<String>();
    server_data.clients.lock().await.insert(key.clone());
    server_data
        .client_event_buffers
        .insert(key.clone(), EventBuffer::new());
    debug!("Added event buffer with id \"{key}\"");

    EventStream! {
        loop {
            yield server_data.client_event_buffers.get(&key).unwrap().pop().await;
        }
    }
}

#[launch]
fn rocket() -> _ {
    //setup_logger();
    rocket::build()
        .manage(server_data::ServerData::default())
        .mount("/", routes![files, events, show_ui, show_login])
        .mount(
            "/groups/",
            routes![
                group::new_group,
                group::get_all_groups,
                group::set_points,
                group::set_answer
            ],
        )
        .mount(
            "/questions/",
            routes![
                questions::load_questions,
                questions::current_question,
                questions::next_question,
                questions::results,
                questions::show_answers,
                questions::show_points,
                questions::show_solution
            ],
        )
        .mount(
            "/display",
            routes![display::show_display],
        )
        .attach(Template::fairing())
}

#[get("/")]
pub async fn show_ui(server_data: &State<ServerData>) -> Template {
    let questions = &server_data.questions.lock().await;
    if questions.is_empty() {
        return Template::render(
            "ui/estimate",
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
            "ui/normal",
            context! {
                question,
                answers,
                solution,
                letters
            },
        ),
        Question::Estimate { question, solution } => Template::render(
            "ui/estimate",
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
            "ui/sort",
            context! {
                question,
                answers,
                solution,
                letters
            },
        ),
    }
}

#[get("/login")]
pub async fn show_login() -> Template {
    Template::render("ui/login", context! {})
}