#![allow(clippy::result_large_err)]

use crossbeam::queue::ArrayQueue;
use rocket::{
    response::stream::Event,
    tokio::sync::{Mutex, Semaphore},
};
use serde::Serialize;
use std::collections::hash_map::Entry;
use std::sync::atomic::Ordering;
use std::{collections::HashMap, str::FromStr, sync::atomic::AtomicUsize};

static NORMAL_POINTS: isize = 1;
static SORT_POINTS: isize = 1;
static ESTIMATE_1_POINTS: isize = 2;
static ESTIMATE_2_POINTS: isize = 1;

#[derive(Default, Debug)]
pub struct ServerData {
    pub groups: Mutex<HashMap<String, GroupData>>,
    pub questions: Mutex<Vec<Question>>,
    pub current_question: AtomicUsize,
    pub display_buffer: EventBuffer,
    pub client_event_buffers: Mutex<HashMap<String, EventBuffer>>, // nächste frage event -> event_buffer
                                                                   // ui <- nächste frage event
                                                                   // display <- ????
}

impl ServerData {
    pub async fn insert_group(&self, name: &str) {
        self.groups
            .lock()
            .await
            .insert(name.to_owned(), GroupData::default());
    }

    pub fn register_display_event(&self, event: Event) -> Result<(), Event> {
        self.display_buffer.push(event)
    }

    pub async fn set_group_points(&self, name: String, number: isize, set: bool) {
        let mut map = self.groups.lock().await.clone();
        let matches = match map.entry(name.clone()) {
            Entry::Occupied(o) => o,
            _ => panic!("Group not found"),
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
    }
    pub async fn set_group_answer(&self, name: String, answer: Answer) {
        let mut map = self.groups.lock().await;
        let group_data = &mut map.get_mut(&name).expect("group not found");
        group_data.answer = Some(answer);
    }
    pub async fn results(&self) {
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
                            self.set_group_points(entry.0, NORMAL_POINTS, false).await;
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
                            self.set_group_points(entry.0, SORT_POINTS, false).await;
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
                .await;
            let est_2_points = if estimate_list[0].0 == estimate_list[1].0 {
                ESTIMATE_1_POINTS
            } else {
                ESTIMATE_2_POINTS
            };
            let est_g_2 = estimate_list[1].1.clone();
            self.set_group_points(est_g_2, est_2_points, false).await;
            let len_v = estimate_list.len();
            if len_v > 2 {
                let mut i: usize = 2;
                while estimate_list[i].0 == estimate_list[i - 1].0 {
                    let est_g = estimate_list[i].1.clone();
                    self.set_group_points(est_g, est_2_points, false).await;
                    i += 1;
                    if len_v <= i {
                        break;
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct EventBuffer(ArrayQueue<Event>, Semaphore);

impl EventBuffer {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_capacity(_cap: usize) -> Self {
        Self(ArrayQueue::new(16), Semaphore::new(0))
    }

    pub fn push(&self, event: Event) -> Result<(), Event> {
        let res = self.0.push(event);
        self.1.add_permits(1);
        res
    }

    pub async fn pop(&self) -> Event {
        let permit = self.1.acquire().await.unwrap();
        permit.forget();
        self.0.pop().unwrap()
    }
}

impl Default for EventBuffer {
    fn default() -> Self {
        Self(ArrayQueue::new(16), Semaphore::new(0))
    }
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct GroupData {
    pub score: isize,
    pub answer: Option<Answer>,
}

#[derive(Debug, Clone, Serialize)]
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
            Some(val) if val == "normal" || val == "sortier" || val == "schaetzen" => val,
            None | Some(_) => return Err("No question type found"),
        };

        let question = match split.next() {
            Some(val) => val,
            None => return Err("No question text found"),
        };

        match question_type {
            "normal" => {
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

            "schaetzen" => {
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
            "sortier" => {
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
