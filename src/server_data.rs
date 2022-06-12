use std::collections::HashMap;

use serde::Serialize;

#[derive(Default, Debug)]
pub struct ServerData {
    pub groups: HashMap<String, GroupData>,
    pub questions: Vec<Question>,
    pub current_question: usize,
}

impl ServerData {
    pub fn insert_group(&mut self, name: &str) {
        self.groups.insert(name.to_owned(), GroupData::default());
    }
}

#[derive(Debug, Default)]
pub struct GroupData {
    pub score: isize,
    pub answer: Option<Answer>,
}

#[derive(Debug)]
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

#[derive(Debug, Clone, Serialize)]
pub enum Answer {
    Normal(usize),
    Estimate(f64),
    Sort([usize; 4]),
}
