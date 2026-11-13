use crate::server_data::{
    normalize_group_name, send_event, Answer, Question, QuestionState, ServerData, UpdateEvent,
    PICTURE_MODE_FULLSCREEN, PICTURE_MODE_HIDDEN, PICTURE_MODE_INLINE, PICTURE_MODE_TOP,
};

use rocket::{http::Status, serde::json::Json, State};
use serde::Serialize;
use std::sync::atomic::Ordering;

#[post("/load", format = "text/plain", data = "<questions>")]
pub async fn load_questions(
    server_data: &State<ServerData>,
    questions: String,
) -> (Status, String) {
    // Editors on Windows like to put a byte order mark in front of the file.
    let text = questions.strip_prefix('\u{feff}').unwrap_or(&questions);

    let mut question_vec = Vec::new();
    for (line_idx, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match line.parse::<Question>() {
            Ok(question) => question_vec.push(question),
            Err(reason) => {
                return (
                    Status::BadRequest,
                    format!("line {}: {reason}", line_idx + 1),
                )
            }
        }
    }
    let count = question_vec.len();

    {
        let mut questions = server_data.questions.lock().await;
        *questions = question_vec;
        server_data.switch_question(&questions, 0);
    }
    send_event(server_data, UpdateEvent::UpdateQuestions);
    (Status::Ok, count.to_string())
}

#[get("/current")]
pub async fn current_question(
    server_data: &State<ServerData>,
) -> (Status, Json<Option<(usize, Question)>>) {
    let questions = server_data.questions.lock().await;
    let current_question_idx = server_data.current_question.load(Ordering::SeqCst);
    match questions.get(current_question_idx) {
        Some(question) => (
            Status::Ok,
            Json(Some((current_question_idx, question.clone()))),
        ),
        None => (Status::NotAcceptable, Json(None)),
    }
}

/// Everything a page needs to bring itself up to date after it was loaded or
/// after it may have missed events.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupQuestionState {
    /// The answer the group has stored for the current question.
    answer: Option<Answer>,
    question_state: QuestionState,
    /// See `ServerData::question_version`.
    question_version: usize,
    /// Equal to `question_count` when the quiz is finished.
    question_index: usize,
    question_count: usize,
    /// False if no group name was sent or the group does not exist (any more).
    group_exists: bool,
    /// The canonical name of the group, if it exists.
    group_name: Option<String>,
    /// True once answers are no longer accepted for the current question.
    answers_blocked: bool,
    leaderboard_shown: bool,
}

/// The body is the group name (may be empty for the display and the admin
/// page). The route deliberately has no `format`: a request without a
/// Content-Type header must not end up as a 404.
#[post("/state", data = "<group_name>")]
pub async fn current_question_state(
    server_data: &State<ServerData>,
    group_name: Option<String>,
) -> Json<GroupQuestionState> {
    let questions = server_data.questions.lock().await;

    let group_name = group_name
        .as_deref()
        .map(normalize_group_name)
        .filter(|name| !name.is_empty());
    let group = group_name.and_then(|name| {
        server_data
            .groups
            .get(name)
            .map(|entry| (entry.key().clone(), entry.answer.clone()))
    });
    let group_exists = group.is_some();
    let (group_name, answer) = match group {
        Some((name, answer)) => (Some(name), answer),
        None => (None, None),
    };

    Json(GroupQuestionState {
        answer,
        question_state: server_data.question_state.clone(),
        question_version: server_data.question_version.load(Ordering::SeqCst),
        question_index: server_data.current_question.load(Ordering::SeqCst),
        question_count: questions.len(),
        group_exists,
        group_name,
        answers_blocked: server_data.block_answer.load(Ordering::SeqCst),
        leaderboard_shown: server_data.leaderboard_shown.load(Ordering::SeqCst),
    })
}

/// Goes to the next question. After the last question the quiz is "finished"
/// (the pages show the waiting screen); from there this does nothing.
///
/// `from` is the index the caller believes to be current. If it is given and
/// wrong, nothing happens, so a double click or a retried request cannot skip
/// a question.
#[get("/next?<from>")]
pub async fn next_question(server_data: &State<ServerData>, from: Option<usize>) -> Status {
    {
        let questions = server_data.questions.lock().await;
        let current_question_idx = server_data.current_question.load(Ordering::SeqCst);
        if from.is_some_and(|from| from != current_question_idx) {
            return Status::Conflict;
        }
        if current_question_idx >= questions.len() {
            return Status::Ok;
        }
        server_data.switch_question(&questions, current_question_idx + 1);
    }
    send_event(server_data, UpdateEvent::UpdateQuestions);
    Status::Ok
}

/// Goes to the question with the given (zero-based) index. Setting the current
/// question again resets it: answers are cleared and answering is reopened.
#[post("/set", format = "text/plain", data = "<question_num>")]
pub async fn set_question(
    server_data: &State<ServerData>,
    question_num: String,
) -> (Status, String) {
    let Ok(value) = question_num.trim().parse::<usize>() else {
        return (
            Status::BadRequest,
            format!("\"{}\" is not a question number", question_num.trim()),
        );
    };
    {
        let questions = server_data.questions.lock().await;
        if value >= questions.len() {
            return (
                Status::BadRequest,
                format!(
                    "question {value} does not exist ({} questions are loaded, numbering starts at 0)",
                    questions.len()
                ),
            );
        }
        server_data.switch_question(&questions, value);
    }
    send_event(server_data, UpdateEvent::UpdateQuestions);
    (Status::Ok, format!("now at question {value}"))
}

/// Awards the points for the current question (once) and shows the solution.
async fn reveal_solution(server_data: &State<ServerData>) -> Status {
    // Holding the lock for the whole operation keeps a concurrent question
    // change or a second reveal from interleaving with the scoring.
    let questions = server_data.questions.lock().await;
    let current_question_idx = server_data.current_question.load(Ordering::SeqCst);
    let Some(question) = questions.get(current_question_idx) else {
        return Status::NotFound;
    };

    let newly_recorded = !server_data.question_state.mark_results_recorded();
    if newly_recorded {
        server_data.record_results(question);
    }
    // Once the solution is visible, answers are closed as well.
    server_data.block_answer.store(true, Ordering::SeqCst);
    server_data.question_state.show_solution();

    if newly_recorded {
        send_event(server_data, UpdateEvent::UpdateGroups);
    }
    send_event(server_data, UpdateEvent::ShowSolution);
    Status::Ok
}

#[get("/results")]
pub async fn results(server_data: &State<ServerData>) -> Status {
    reveal_solution(server_data).await
}

#[get("/show_solution")]
pub async fn show_solution(server_data: &State<ServerData>) -> Status {
    reveal_solution(server_data).await
}

#[get("/allow_results_entry")]
pub async fn allow_results_entry(server_data: &State<ServerData>) -> Status {
    let _questions = server_data.questions.lock().await;
    server_data.question_state.set_results_recorded(false);
    Status::Ok
}

#[get("/show_answers")]
pub async fn show_answers(server_data: &State<ServerData>) -> Status {
    {
        let _questions = server_data.questions.lock().await;
        // Block first: no answer may slip in after the answers became visible.
        server_data.block_answer.store(true, Ordering::SeqCst);
        server_data.question_state.show_answers();
    }
    send_event(server_data, UpdateEvent::ShowAnswers);
    Status::Ok
}

#[get("/show_points")]
pub async fn show_points(server_data: &State<ServerData>) -> Status {
    send_event(server_data, UpdateEvent::ShowPoints);
    Status::Ok
}

async fn change_picture_mode(
    server_data: &State<ServerData>,
    mode: usize,
    event: UpdateEvent,
) -> Status {
    {
        let questions = server_data.questions.lock().await;
        let current_question_idx = server_data.current_question.load(Ordering::SeqCst);
        let Some(question) = questions.get(current_question_idx) else {
            return Status::NotFound;
        };
        if !question.has_display_picture() {
            return Status::NotFound;
        }
        if question.display_picture_requires_release()
            && !server_data.question_state.answer_is_shown()
        {
            return Status::Forbidden;
        }

        if mode != PICTURE_MODE_HIDDEN {
            server_data.question_state.release_picture();
        }
        server_data.question_state.set_picture_mode(mode);
    }
    send_event(server_data, event);
    Status::Ok
}

#[get("/show_picture_inline")]
pub async fn show_picture_inline(server_data: &State<ServerData>) -> Status {
    change_picture_mode(
        server_data,
        PICTURE_MODE_INLINE,
        UpdateEvent::ShowPictureInline,
    )
    .await
}

#[get("/show_picture_top")]
pub async fn show_picture_top(server_data: &State<ServerData>) -> Status {
    change_picture_mode(server_data, PICTURE_MODE_TOP, UpdateEvent::ShowPictureTop).await
}

#[get("/show_picture_fullscreen")]
pub async fn show_picture_fullscreen(server_data: &State<ServerData>) -> Status {
    change_picture_mode(
        server_data,
        PICTURE_MODE_FULLSCREEN,
        UpdateEvent::ShowPictureFullscreen,
    )
    .await
}

#[get("/hide_picture")]
pub async fn hide_picture(server_data: &State<ServerData>) -> Status {
    change_picture_mode(server_data, PICTURE_MODE_HIDDEN, UpdateEvent::HidePicture).await
}
