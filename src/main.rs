use std::{
    ops::Deref,
    path::{Path, PathBuf},
    sync::{atomic::Ordering, Arc},
};

use rand::{distributions::Alphanumeric, Rng};
use rocket::{
    fairing::{Fairing, Info, Kind},
    fs::NamedFile,
    http::{Header, Status},
    response::stream::{Event, EventStream},
    tokio::select,
    Request, Response, Shutdown, State,
};
use rocket_dyn_templates::{context, Template};
use server_data::{send_event, UpdateEvent};
use simplelog::{ConfigBuilder, TermLogger};

use crate::server_data::{EventBuffer, Question, ServerData, EVENT_BUFFER_KEY_LENGTH};

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
/// I am stupid and don't know how webdev works so this is here.
#[options("/<_..>")]
async fn cors_fix() -> Status {
    Status::Ok
}

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Attaching CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, PATCH, OPTIONS",
        ));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
        debug!("Attached Cors Headers to response");
    }
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

/// Wrapper around an event buffer that automatically disconects it once it goes out of scope
struct BufferHandle(Arc<EventBuffer>);

impl Deref for BufferHandle {
    type Target = Arc<EventBuffer>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for BufferHandle {
    fn drop(&mut self) {
        self.0.disconnect();
    }
}

#[get("/events")]
pub async fn events(
    server_data: &State<ServerData>,
    mut shutdown: Shutdown,
) -> EventStream![Event + '_] {
    let key = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(EVENT_BUFFER_KEY_LENGTH)
        .map(char::from)
        .collect::<String>();
    let buffer = Arc::new(EventBuffer::with_capacity(4));
    server_data
        .client_event_buffers
        .insert(key.clone(), Arc::clone(&buffer));
    debug!("Added event buffer with id \"{key}\"");
    let buffer = BufferHandle(buffer);

    EventStream! {
        loop {
            select! {
                event = buffer.pop() => { yield event; }
                _ = &mut shutdown => {
                    break;
                }
            }
        }
    }
}

#[launch]
fn rocket() -> _ {
    //setup_logger();
    rocket::build()
        .manage(server_data::ServerData::default())
        .mount(
            "/",
            routes![
                files,
                events,
                show_ui,
                show_login,
                cors_fix,
                show_admin,
                toggle_leaderboard
            ],
        )
        .mount(
            "/groups/",
            routes![
                group::new_group,
                group::get_all_groups,
                group::set_score,
                group::add_score,
                group::set_answer,
                group::del_group
            ],
        )
        .mount(
            "/questions/",
            routes![
                questions::load_questions,
                questions::current_question,
                questions::current_question_state,
                questions::next_question,
                questions::results,
                questions::show_answers,
                questions::show_points,
                questions::show_solution,
                questions::set_question
            ],
        )
        .mount(
            "/display",
            routes![display::show_display, display::show_leaderboard],
        )
        .attach(Template::fairing())
        .attach(CORS)
}

#[get("/admin_endpoint_sehr_billig")]
pub async fn show_admin() -> Template {
    Template::render("admin/index", context! {})
}

#[get("/")]
pub async fn show_ui(server_data: &State<ServerData>) -> Template {
    let questions = &server_data.questions.lock().await;
    if questions.is_empty() {
        return Template::render("ui/waiting", context! { question: "" });
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

#[get("/toggle_leaderboard")]
pub async fn toggle_leaderboard(server_data: &State<ServerData>) -> Status {
    send_event(server_data, UpdateEvent::ToggleLeaderboard).await;
    Status::Ok
}
