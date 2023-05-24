#![allow(clippy::result_large_err)]

use chashmap::CHashMap;
use crossbeam::queue::ArrayQueue;
use log::{info, warn};
use rocket::{
    response::stream::Event,
    tokio::sync::{Mutex, Semaphore},
    State,
};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::Entry;
use std::{
    collections::HashMap, str::FromStr, sync::atomic::AtomicBool, sync::atomic::AtomicUsize,
};
use std::{collections::HashSet, fmt::Display, sync::atomic::Ordering};
use thiserror::Error;

const NORMAL_POINTS: isize = 1;
const SORT_POINTS: isize = 1;
const ESTIMATE_1_POINTS: isize = 2;
const ESTIMATE_2_POINTS: isize = 1;

pub const EVENT_BUFFER_KEY_LENGTH: usize = 32;

const NORMAL_NAME: &str = "normal";
const ESTIMATE_NAME: &str = "schaetzen";
const SORT_NAME: &str = "sortier";

const QUESTION_TYPES: [&str; 3] = [NORMAL_NAME, ESTIMATE_NAME, SORT_NAME];

#[derive(Default, Debug)]
pub struct ServerData {
    pub groups: Mutex<HashMap<String, GroupData>>,
    pub questions: Mutex<Vec<Question>>,
    pub current_question: AtomicUsize,
    pub block_answer: AtomicBool,
    pub clients: Mutex<HashSet<String>>,
    pub client_event_buffers: CHashMap<String, EventBuffer>, // nächste frage event -> event_buffer
                                                             // ui <- nächste frage event
                                                             // display <- ????
}

#[derive(Error, Debug, PartialEq, Clone, Hash)]
pub enum GroupError {
    #[error("group '{0}' not found")]
    NotFound(String),
    #[error("group with name '{0}' already exists")]
    Duplicate(String),
}

impl ServerData {
    pub async fn insert_group(&self, name: &str) -> Result<(), GroupError> {
        match self.groups.lock().await.entry(name.to_owned()) {
            Entry::Vacant(v) => {
                v.insert(GroupData::default());
                Ok(())
            }
            Entry::Occupied(_) => Err(GroupError::Duplicate(name.to_owned())),
        }
    }

    pub async fn delete_group(&self, name: &str) {
        let mut map = self.groups.lock().await;

        if map.remove(name).is_some() {
            info!("group with name '{}' deleted.", name);
        } else {
            info!("group with name '{}' not found.", name);
        }
    }

    pub async fn set_group_points(
        &self,
        name: impl AsRef<str>,
        number: isize,
        set: bool,
    ) -> Result<(), GroupError> {
        let name = name.as_ref().to_owned();
        let mut map = self.groups.lock().await.clone();
        let matches = match map.entry(name.clone()) {
            Entry::Occupied(o) => o,
            _ => return Err(GroupError::NotFound(name)),
        };
        let g_data: &GroupData = matches.get();
        let mut new_group_data = g_data.clone();
        let score = if !set {
            new_group_data.score + number
        } else {
            number
        };

        new_group_data.score = score;
        self.groups
            .lock()
            .await
            .entry(name)
            .and_modify(|e| *e = new_group_data);

        Ok(())
    }

    pub async fn set_group_answer(
        &self,
        name: impl AsRef<str>,
        answer: Answer,
    ) -> Result<(), GroupError> {
        let name = name.as_ref();
        let mut map = self.groups.lock().await;
        let Some(group_data) = &mut map.get_mut(name) else {
            return Err(GroupError::NotFound(name.to_owned()));
        };
        group_data.answer = Some(answer);
        Ok(())
    }

    pub async fn clear_group_answers(&self) {
        self.groups.lock().await.values_mut().for_each(|group| {
            group.answer = None;
        })
    }

    pub async fn results(&self) -> Result<(), GroupError> {
        if self.current_question.load(Ordering::Relaxed) >= self.questions.lock().await.len() {
            panic!("Error by loading question");
        }
        let current_question_idx = self.current_question.load(Ordering::Relaxed);
        let question = self.questions.lock().await[current_question_idx].clone();

        let map = self.groups.lock().await.clone();
        let mut estimate_list: Vec<(f64, String)> = Vec::new();
        for entry in map {
            let answ: Answer = match entry.1.answer {
                Some(a) => a,
                None => continue,
            };
            match &question {
                Question::Normal {
                    question: _,
                    answers: _,
                    solution,
                } => match answ {
                    Answer::Normal(ans) => {
                        if *solution == ans {
                            self.set_group_points(entry.0, NORMAL_POINTS, false).await?;
                        }
                    }
                    _ => continue,
                },
                Question::Estimate {
                    question: _,
                    solution,
                } => match answ {
                    Answer::Estimate(ans) => {
                        estimate_list.push(((solution - ans).abs(), entry.0));
                    }
                    _ => continue,
                },
                Question::Sort {
                    question: _,
                    answers: _,
                    solution,
                } => match answ {
                    Answer::Sort(ans) => {
                        if *solution == ans {
                            self.set_group_points(entry.0, SORT_POINTS, false).await?;
                        }
                    }
                    _ => continue,
                },
            }
        }
        if !estimate_list.is_empty() {
            estimate_list.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            let est_g_1 = estimate_list[0].1.clone();
            self.set_group_points(est_g_1, ESTIMATE_1_POINTS, false)
                .await?;
            let est_2_points = if estimate_list[0].0 == estimate_list[1].0 {
                ESTIMATE_1_POINTS
            } else {
                ESTIMATE_2_POINTS
            };
            let est_g_2 = estimate_list[1].1.clone();
            self.set_group_points(est_g_2, est_2_points, false).await?;
            let len_v = estimate_list.len();
            if len_v > 2 {
                let mut i: usize = 2;
                while estimate_list[i].0 == estimate_list[i - 1].0 {
                    let est_g = estimate_list[i].1.clone();
                    self.set_group_points(est_g, est_2_points, false).await?;
                    i += 1;
                    if len_v <= i {
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct EventBuffer(ArrayQueue<Event>, Semaphore);

impl EventBuffer {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_capacity(cap: usize) -> Self {
        log::info!("Created event queue with capacity {cap}");
        Self(ArrayQueue::new(cap), Semaphore::new(0))
    }

    pub fn push(&self, event: Event) -> Result<(), Event> {
        let res = self.0.push(event);
        self.1.add_permits(1);
        log::info!("Added event to queue. Now {} events stored", self.0.len());
        res
    }

    pub async fn pop(&self) -> Event {
        let permit = self.1.acquire().await.unwrap();
        permit.forget();
        log::info!(
            "Removed event from queue. Now {} events stored",
            self.0.len() - 1
        );
        self.0.pop().unwrap()
    }
}

impl Default for EventBuffer {
    fn default() -> Self {
        Self::with_capacity(16)
    }
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct GroupData {
    pub score: isize,
    pub answer: Option<Answer>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum Question {
    Normal {
        question: String,
        answers: [String; 4],
        solution: usize,
    },
    Estimate {
        question: String,
        solution: f64,
    },
    Sort {
        question: String,
        answers: [String; 4],
        solution: [usize; 4],
    },
}

impl Question {
    pub fn normal(question: &str, answers: &[String], solution: usize) -> Self {
        if answers.len() != 4 {
            panic!("Wrong amount of answers")
        }
        Question::Normal {
            question: question.to_owned(),
            answers: [
                answers[0].clone(),
                answers[1].clone(),
                answers[2].clone(),
                answers[3].clone(),
            ],
            solution,
        }
    }

    pub fn estimate(question: &str, solution: f64) -> Self {
        Question::Estimate {
            question: question.to_owned(),
            solution,
        }
    }

    pub fn sort(question: &str, answers: &[String], solution: &[usize]) -> Self {
        if answers.len() != 4 {
            panic!("Wrong amount of answers")
        }
        if solution.len() != 4 {
            panic!("Wrong amount of values in solution")
        }
        Question::Sort {
            question: question.to_owned(),
            answers: [
                answers[0].clone(),
                answers[1].clone(),
                answers[2].clone(),
                answers[3].clone(),
            ],
            solution: [solution[0], solution[1], solution[2], solution[3]],
        }
    }
}

impl FromStr for Question {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split('#');

        let question_type = match split.next() {
            Some(val) if QUESTION_TYPES.contains(&val) => val,
            None | Some(_) => return Err("No question type found"),
        };

        let question = match split.next() {
            Some(val) => val,
            None => return Err("No question text found"),
        };

        match question_type {
            NORMAL_NAME => {
                let mut answers: [String; 4] = Default::default();
                for answer in answers.iter_mut() {
                    *answer = match split.next() {
                        Some(val) => val.to_owned(),
                        None => return Err("Answer not found"),
                    }
                }

                let solution = {
                    let so = match split.next() {
                        Some(val) => val,
                        None => return Err("No solution found"),
                    };

                    match so {
                        "a" => 0,
                        "b" => 1,
                        "c" => 2,
                        "d" => 3,
                        _ => return Err("Invalid solution"),
                    }
                };

                Ok(Question::normal(question, &answers, solution))
            }

            ESTIMATE_NAME => {
                let solution = {
                    let so = match split.next() {
                        Some(val) => val,
                        None => return Err("No solution found"),
                    };

                    match so.parse::<f64>() {
                        Ok(val) => val,
                        _ => return Err("Estimate solution not a number"),
                    }
                };

                Ok(Question::estimate(question, solution))
            }
            SORT_NAME => {
                let mut answers: [String; 4] = Default::default();
                for answer in answers.iter_mut() {
                    *answer = match split.next() {
                        Some(val) => val.to_owned(),
                        None => return Err("Answer not found"),
                    }
                }
                let mut solutions = [0usize; 4];
                for solution in solutions.iter_mut() {
                    let so = match split.next() {
                        Some(val) => val,
                        None => return Err("Answer not found"),
                    };

                    *solution = match so {
                        "a" => 0,
                        "b" => 1,
                        "c" => 2,
                        "d" => 3,
                        _ => return Err("Invalid solution"),
                    };
                }

                Ok(Question::sort(question, &answers, &solutions))
            }
            _ => Err("?"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum Answer {
    Normal(usize),
    Estimate(f64),
    Sort([usize; 4]),
}

impl Answer {
    pub fn try_parse_answer(
        answer_type: impl AsRef<str>,
        answer_string: impl AsRef<str>,
    ) -> Result<Answer, String> {
        let s = answer_type.as_ref();
        let answer_string = answer_string.as_ref();

        fn convert_letters_to_numbers(letters: &str) -> Result<[usize; 4], String> {
            if letters.len() != 4 {
                return Err(format!(
                    "could not convert \"{letters}\" into a sorting answer"
                ));
            }
            let mut numbers = [0; 4];

            for (i, letter) in letters.chars().map(|c| c.to_ascii_lowercase()).enumerate() {
                match letter {
                    'a' => numbers[i] = 0,
                    'b' => numbers[i] = 1,
                    'c' => numbers[i] = 2,
                    'd' => numbers[i] = 3,
                    _ => return Err(format!("invalid letter: {letter}")),
                }
            }

            Ok(numbers)
        }
        match s {
            NORMAL_NAME => answer_string
                .parse()
                .map_err(|_| format!("could not parse answer \"{s}\" as a number"))
                .map(Answer::Normal),
            ESTIMATE_NAME => answer_string
                .parse()
                .map_err(|_| format!("could not parse answer \"{s}\" as a number"))
                .map(Answer::Estimate),
            SORT_NAME => convert_letters_to_numbers(answer_string).map(Answer::Sort),
            _ => Err(format!("invalid answer type: \"{s}\"")),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateEvent {
    /// Update the questions shown in the UI
    UpdateQuestions,
    /// Update the groups shown in the UI
    UpdateGroups,
    /// Show the answers the groups have given in the display
    ShowAnswers,
    /// Shows the solution to the current question
    ShowSolution,
    ShowPoints,
    ShowScore,
}

impl Display for UpdateEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use UpdateEvent::*;
        write!(
            f,
            "{}",
            match *self {
                UpdateQuestions => "question",
                UpdateGroups => "groups",
                ShowAnswers => "show_answers",
                ShowSolution => "show_solution",
                ShowPoints => "show_points",
                ShowScore => "show_score",
            }
        )
    }
}

pub async fn send_event(server_data: &State<ServerData>, event: UpdateEvent) {
    info!("sending event with text: {event}");

    // The buffers we want to delete since they don't seem to be connected anymore
    let mut to_delete = vec![];
    // Write the event into every event buffer available
    for id in server_data.clients.lock().await.iter() {
        let Some(buffer) = server_data.client_event_buffers.get(id) else {
            warn!("buffer corresponding to id {id} not found");
            continue;
        };
        if buffer.push(Event::data(event.to_string())).is_err() {
            warn!("event dropped for buffer {id}! the queue is full. assuming the client is not connected and deleting the buffer");
            to_delete.push(id.clone());
        }
    }
    let mut clients = server_data.clients.lock().await;
    for name in to_delete {
        server_data.client_event_buffers.remove(&name);
        clients.remove(&name);
    }
}
