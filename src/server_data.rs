

pub struct ServerData {
    groups: Vec<Group>,
    questions: Vec<Question>,
}

pub struct Group { 
    name: String,
    score: isize,
}

impl Group {
    pub fn new(name: &str) -> Self {
        Group {
            name: name.to_owned(),
            score: 0
        }
    }
}

pub enum Question {
    Normal {
        question: String,
        possibilities: [String; 4],
        correct_answer: usize
    },
    Estimate {
        question: String,
        answer: f64
    },
    Sort {
        question: String,
        possibilities: [String; 4],
        correct_order: [usize; 4]
    }
}



