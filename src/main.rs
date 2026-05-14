use std::{
    ops::Deref,
    path::{Path, PathBuf},
    sync::{atomic::Ordering, Arc},
};

use rand::{distr::Alphanumeric,  Rng};
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
    NamedFile::open(Path::new("frontend").join(file))
        .await
        .ok()
}

#[get("/pictures/<file..>")]
async fn picture_files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("pictures").join(file))
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

#[allow(unused)]
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
    let key = rand::rng()
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
                picture_files,
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
                questions::allow_results_entry,
                questions::set_question,
                questions::show_picture_inline,
                questions::show_picture_top,
                questions::show_picture_fullscreen,
                questions::hide_picture
            ],
        )
        .mount(
            "/display",
            routes![display::show_display, display::show_leaderboard],
        )
        .attach(Template::fairing())
        .attach(CORS)
}

#[get("/admin_billig")]
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
    let image_before = question.ui_image_before();
    let image_after = question.ui_image_after();

    let letters = ['A', 'B', 'C', 'D', 'E', 'F'];

    debug!("{}", serde_json::to_string(&question).unwrap());

    match question {
        Question::Normal {
            question,
            answers,
            solution,
            ..
        } => Template::render(
            "ui/normal",
            context! {
                question,
                answers,
                solution,
                letters,
                image_before,
                image_after
            },
        ),
        Question::Estimate {
            question,
            solution,
            ..
        } => Template::render(
            "ui/estimate",
            context! {
                question,
                solution,
                image_before,
                image_after,
            },
        ),
        Question::Sort {
            question,
            answers,
            solution,
            ..
        } => Template::render(
            "ui/sort",
            context! {
                question,
                answers,
                solution,
                letters,
                image_before,
                image_after
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
                "ui/zuordnen",
                context! {
                    question,
                    answers,
                    solution_letters,
                    letters,
                    image_before,
                    image_after
                },
            )
        }
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
