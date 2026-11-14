use std::{
    path::{Path, PathBuf},
    sync::atomic::Ordering,
    time::{Duration, UNIX_EPOCH},
};

use rocket::{
    fairing::{Fairing, Info, Kind},
    fs::NamedFile,
    http::{Header, Status},
    response::{
        self,
        stream::{Event, EventStream},
        Responder,
    },
    tokio::{
        select,
        sync::broadcast::error::RecvError,
        time::{interval, MissedTickBehavior},
    },
    Request, Response, Shutdown, State,
};
use rocket_dyn_templates::{context, Template};
use server_data::{send_event, UpdateEvent};
use simplelog::{ConfigBuilder, TermLogger};

use crate::server_data::{Question, ServerData, MAX_GROUP_NAME_CHARS};

#[macro_use]
extern crate rocket;

mod client;
mod display;
mod group;
mod questions;
mod server_data;

/// How often every event stream sends a "ping" event. The pages treat a
/// connection that stays silent for a few of these intervals as dead and open
/// a new one (`SILENCE_LIMIT_MS` in frontend/res/common.js, keep it at about
/// three times this value).
const EVENT_PING_INTERVAL: Duration = Duration::from_secs(2);

/// A static file that browsers may keep but have to revalidate on every use.
///
/// `NamedFile` alone sends nothing a browser could revalidate with, so every
/// page load would download every picture again. With the `ETag` an unchanged
/// file costs one small 304 answer, and a file that was replaced under the
/// same name is picked up immediately.
struct StaticFile {
    file: NamedFile,
    etag: String,
}

impl StaticFile {
    async fn open(path: PathBuf) -> Option<Self> {
        let file = NamedFile::open(path).await.ok()?;
        let metadata = file.file().metadata().await.ok()?;
        // Size and modification time identify the content well enough.
        let modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |duration| duration.as_nanos());
        let etag = format!("\"{:x}-{:x}\"", metadata.len(), modified);
        Some(Self { file, etag })
    }
}

impl<'r> Responder<'r, 'static> for StaticFile {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        let browser_has_it = request
            .headers()
            .get("If-None-Match")
            .flat_map(|value| value.split(','))
            .map(|tag| tag.trim().trim_start_matches("W/"))
            .any(|tag| tag == "*" || tag == self.etag);

        let mut response = if browser_has_it {
            Response::build().status(Status::NotModified).finalize()
        } else {
            self.file.respond_to(request)?
        };
        response.set_header(Header::new("ETag", self.etag));
        response.set_header(Header::new("Cache-Control", "no-cache"));
        Ok(response)
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
///
/// "ping" events are sent all the time, so that a page can tell a quiet
/// connection from a dead one (see `EVENT_PING_INTERVAL`).
#[get("/events")]
pub fn events(server_data: &State<ServerData>, mut shutdown: Shutdown) -> EventStream![] {
    // Subscribe here and not inside the stream: the stream body only starts
    // running when the response is sent, and events in between would be lost.
    let mut receiver = server_data.events.subscribe();

    EventStream! {
        // Tell the browser to reconnect quickly after a lost connection.
        yield Event::retry(Duration::from_secs(1));
        let mut ping = interval(EVENT_PING_INTERVAL);
        ping.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            let event = select! {
                message = receiver.recv() => match message {
                    Ok(event) => Event::data(event.to_string()),
                    // This client was too slow and events were dropped for it.
                    Err(RecvError::Lagged(_)) => Event::data("resync"),
                    Err(RecvError::Closed) => break,
                },
                _ = ping.tick() => Event::data("ping"),
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
    Template::render(
        "ui/login",
        context! { max_group_name_chars: MAX_GROUP_NAME_CHARS },
    )
}

/// Switches the display between the question and the leaderboard.
///
/// `show` is the wanted state. With it the request can be repeated safely;
/// without it the state is flipped.
#[get("/toggle_leaderboard?<show>")]
pub async fn toggle_leaderboard(server_data: &State<ServerData>, show: Option<&str>) -> Status {
    // Parsed here and not by Rocket: Rocket turns a value that does not parse
    // into `None`, which would flip the state instead of refusing.
    let show = match show.map(str::parse::<bool>) {
        None => None,
        Some(Ok(show)) => Some(show),
        Some(Err(_)) => return Status::BadRequest,
    };
    match show {
        Some(show) => server_data.leaderboard_shown.store(show, Ordering::SeqCst),
        None => {
            server_data.leaderboard_shown.fetch_xor(true, Ordering::SeqCst);
        }
    }
    send_event(server_data, UpdateEvent::ToggleLeaderboard);
    Status::Ok
}
