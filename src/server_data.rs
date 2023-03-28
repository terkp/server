use std::{
    collections::{HashMap, VecDeque},
    str::FromStr,
    sync::atomic::AtomicUsize,
};
use std::collections::hash_map::Entry;
use crossbeam::queue::ArrayQueue;
use rocket::{tokio::sync::{Mutex, Semaphore}, response::stream::Event};
use serde::Serialize;

#[derive(Default, Debug)]
pub struct ServerData {
    pub groups: Mutex<HashMap<String, GroupData>>,
    pub questions: Mutex<Vec<Question>>,
    pub current_question: AtomicUsize,
    pub display_buffer: EventBuffer,
    // nächste frage event -> event_buffer
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
    pub async fn set_group_points(&self,name:String,number: isize,set:bool) {
        let name_s = name.clone();
        let mut map = self.groups.lock().await.clone();
        let matches = match map.entry(name_s) {
            Entry::Occupied(o) => o,
            _ => panic!("Group not found"),
        };
        let G_data: &GroupData = matches.get();
        let mut new_GroupData = G_data.clone();
        let score:isize;
        if set == false{
            score = new_GroupData.score + number;
            
        }
        else {
            score = number;
        }
        new_GroupData.score = score;
        self.groups.lock().await.entry(name).and_modify(|e| {*e =new_GroupData});
        
    }
    pub async fn set_group_answer(&self,name:String, answer:Answer) {
        let name_s = name.clone();
        let mut map = self.groups.lock().await.clone();
        let matches = match map.entry(name_s) {
            Entry::Occupied(o) => o,
            _ => panic!("Group not found"),
        };
        let G_data: &GroupData = matches.get();
        let mut new_GroupData = G_data.clone();
        new_GroupData.answer = Some(answer);
        self.groups.lock().await.entry(name).and_modify(|e| {*e =new_GroupData});
    }
}

#[derive(Debug)]
pub struct EventBuffer(ArrayQueue<Event>, Semaphore);

impl EventBuffer {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_capacity(cap: usize) -> Self {
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
        let mut split = s.split("#");

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
                for i in 0..4 {
                    answers[i] = match split.next() {
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
                for i in 0..4 {
                    answers[i] = match split.next() {
                        Some(val) => val.to_owned(),
                        None => return Err("Answer not found"),
                    }
                }
                let mut solutions = [0usize; 4];
                for i in 0..4 {
                    let so = match split.next() {
                        Some(val) => val,
                        None => return Err("Answer not found"),
                    };

                    solutions[i] = match so {
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
