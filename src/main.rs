use std::{
    path::{Path, PathBuf},
    sync::atomic::Ordering,
    time::Duration,
};

use rocket::{
    fairing::{Fairing, Info, Kind},
    fs::NamedFile,
    http::{Header, Status},
    response::stream::{Event, EventStream},
    tokio::{select, sync::broadcast::error::RecvError},
    Request, Response, Shutdown, State,
};
use rocket_dyn_templates::{context, Template};
use server_data::{send_event, UpdateEvent};
use simplelog::{ConfigBuilder, TermLogger};

use crate::server_data::{Question, ServerData};

#[macro_use]
extern crate rocket;

mod client;
mod display;
mod group;
mod questions;
mod server_data;

/// How long browsers may reuse a picture without asking again. `NamedFile`
/// sends no validators, so without this every page load downloads every
/// picture again.
const CACHE_PICTURES: &str = "public, max-age=3600";
/// Scripts, styles and everything else are always fetched fresh, so a changed
/// script is never mixed with an old page.
const CACHE_NOTHING: &str = "no-cache";

/// A file together with a `Cache-Control` header.
#[derive(Responder)]
struct StaticFile {
    file: NamedFile,
    cache_control: Header<'static>,
}

impl StaticFile {
    async fn open(path: PathBuf) -> Option<Self> {
        let is_picture = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .is_some_and(|extension| {
                matches!(
                    extension.as_str(),
                    "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "ico"
                )
            });
        let cache_control = if is_picture {
            CACHE_PICTURES
        } else {
            CACHE_NOTHING
        };
        Some(Self {
            file: NamedFile::open(path).await.ok()?,
            cache_control: Header::new("Cache-Control", cache_control),
        })
    }
}

#[get("/<file..>")]
async fn files(file: PathBuf) -> Option<StaticFile> {
    StaticFile::open(Path::new("frontend").join(file)).await
}

#[get("/pictures/<file..>")]
async fn picture_files(file: PathBuf) -> Option<StaticFile> {
    StaticFile::open(Path::new("pictures").join(file)).await
}

/// A cheap request the pages use to find out whether the server is reachable
/// before they reload or navigate.
#[get("/ping")]
fn ping() -> &'static str {
    "pong"
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

/// The server-sent event stream every page listens to.
///
/// Events are only hints ("something changed"); the pages fetch the actual
/// state from `/questions/state` and `/groups/get`. A client that missed
/// events is sent "resync" and then does the same.
#[get("/events")]
pub fn events(server_data: &State<ServerData>, mut shutdown: Shutdown) -> EventStream![] {
    // Subscribe here and not inside the stream: the stream body only starts
    // running when the response is sent, and events in between would be lost.
    let mut receiver = server_data.events.subscribe();

    EventStream! {
        // Tell the browser to reconnect quickly after a lost connection.
        yield Event::retry(Duration::from_secs(1));
        loop {
            let event = select! {
                message = receiver.recv() => match message {
                    Ok(event) => Event::data(event.to_string()),
                    // This client was too slow and events were dropped for it.
                    Err(RecvError::Lagged(_)) => Event::data("resync"),
                    Err(RecvError::Closed) => break,
                },
                _ = &mut shutdown => break,
            };
            yield event;
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
                ping,
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
    // Index and version are read under the questions lock, so the page never
    // carries a version that is newer than its content.
    let questions = &server_data.questions.lock().await;
    let question_version = server_data.question_version.load(Ordering::SeqCst);
    let current_question_idx = server_data.current_question.load(Ordering::SeqCst);

    // No questions loaded, or the quiz is finished.
    let Some(question) = questions.get(current_question_idx) else {
        return Template::render("ui/waiting", context! { question: "", question_version });
    };
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
                image_after,
                question_version
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
                question_version
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
                image_after,
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
                "ui/zuordnen",
                context! {
                    question,
                    answers,
                    solution_letters,
                    letters,
                    image_before,
                    image_after,
                    question_version
                },
            )
        }
    }
}

#[get("/login")]
pub async fn show_login() -> Template {
    Template::render("ui/login", context! {})
}

/// Switches the display between the question and the leaderboard.
///
/// `show` is the wanted state. With it the request can be repeated safely;
/// without it the state is flipped.
#[get("/toggle_leaderboard?<show>")]
pub async fn toggle_leaderboard(server_data: &State<ServerData>, show: Option<bool>) -> Status {
    match show {
        Some(show) => server_data.leaderboard_shown.store(show, Ordering::SeqCst),
        None => {
            server_data.leaderboard_shown.fetch_xor(true, Ordering::SeqCst);
        }
    }
    send_event(server_data, UpdateEvent::ToggleLeaderboard);
    Status::Ok
}
