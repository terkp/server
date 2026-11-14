#![allow(clippy::result_large_err)]

use dashmap::{mapref::entry::Entry, DashMap};
use log::info;
use rocket::tokio::sync::{broadcast, Mutex};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    path::{Component, Path},
    str::FromStr,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};
use thiserror::Error;

const NORMAL_POINTS: f64 = 2.0;
const SORT_POINTS: f64 = 2.0;
const ESTIMATE_1_POINTS: f64 = 3.0;
const ESTIMATE_2_POINTS: f64 = 2.0;
const ESTIMATE_3_POINTS: f64 = 1.0;
const ASSIGN_POINTS_PER_ITEM: f64 = 0.5;

/// Number of events a slow client may fall behind before it is told to resync.
const EVENT_CHANNEL_CAPACITY: usize = 256;

/// Maximum length of a group name, counted in characters.
pub const MAX_GROUP_NAME_CHARS: usize = 40;

const NORMAL_NAME: &str = "normal";
const ESTIMATE_NAME: &str = "schaetzen";
const SORT_NAME: &str = "sortier";

const ASSIGN_NAME: &str = "zuordnen";

const QUESTION_TYPES: [&str; 4] = [NORMAL_NAME, ESTIMATE_NAME, SORT_NAME, ASSIGN_NAME];

pub const PICTURE_MODE_HIDDEN: usize = 0;
pub const PICTURE_MODE_INLINE: usize = 1;
pub const PICTURE_MODE_TOP: usize = 2;
pub const PICTURE_MODE_FULLSCREEN: usize = 3;

/// The shared state of the quiz.
///
/// The `questions` mutex is also the lock for everything that belongs to the
/// current question: `current_question`, `question_version`, `question_state`
/// and `block_answer` are only changed while it is held, and code that needs a
/// consistent view of them (page rendering, the state endpoint, `set_answer`)
/// reads them while holding it.
#[derive(Debug)]
pub struct ServerData {
    pub groups: DashMap<String, GroupData>,
    pub questions: Mutex<Vec<Question>>,
    /// Index into `questions`. Equal to `questions.len()` once the quiz is finished.
    pub current_question: AtomicUsize,
    /// Changes whenever the current question changes (load, next, set). Seeded
    /// randomly so that a server restart also counts as a change. Pages carry
    /// the version they were rendered with and reload when it no longer matches.
    pub question_version: AtomicUsize,
    pub question_state: QuestionState,
    pub block_answer: AtomicBool,
    /// Whether the display should show the leaderboard instead of the question.
    pub leaderboard_shown: AtomicBool,
    /// Every `/events` connection subscribes to this channel.
    pub events: broadcast::Sender<UpdateEvent>,
}

impl Default for ServerData {
    fn default() -> Self {
        let (events, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        Self {
            groups: DashMap::new(),
            questions: Mutex::new(Vec::new()),
            current_question: AtomicUsize::new(0),
            question_version: AtomicUsize::new(rand::random::<u32>() as usize),
            question_state: QuestionState::default(),
            block_answer: AtomicBool::new(false),
            leaderboard_shown: AtomicBool::new(false),
            events,
        }
    }
}

/// The canonical form of a group name. Every lookup goes through this, so
/// "Team" and "Team " (phone keyboards like to append a space) are one group.
pub fn normalize_group_name(name: &str) -> &str {
    name.trim()
}

/// Normalizes a name for a new group and checks that it is usable.
/// The error text is shown to the player.
pub fn validate_group_name(name: &str) -> Result<&str, String> {
    let name = normalize_group_name(name);
    if name.is_empty() {
        return Err("Der Gruppenname darf nicht leer sein.".to_owned());
    }
    if name.chars().count() > MAX_GROUP_NAME_CHARS {
        return Err(format!(
            "Der Gruppenname darf höchstens {MAX_GROUP_NAME_CHARS} Zeichen lang sein."
        ));
    }
    Ok(name)
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct QuestionState {
    #[serde(rename = "answerIsShown")]
    answer_is_shown: AtomicBool,
    #[serde(rename = "solutionIsShown")]
    solution_is_shown: AtomicBool,
    #[serde(rename = "resultsRecorded")]
    results_recorded: AtomicBool,
    #[serde(rename = "pictureReleased")]
    picture_released: AtomicBool,
    #[serde(rename = "pictureMode")]
    picture_mode: AtomicUsize,
}

impl QuestionState {
    pub fn reset(&self) {
        self.answer_is_shown.store(false, Ordering::Relaxed);
        self.solution_is_shown.store(false, Ordering::Relaxed);
        self.results_recorded.store(false, Ordering::Relaxed);
        self.picture_released.store(false, Ordering::Relaxed);
        self.picture_mode
            .store(PICTURE_MODE_HIDDEN, Ordering::Relaxed);
    }

    pub fn show_answers(&self) {
        self.answer_is_shown.store(true, Ordering::Relaxed)
    }

    pub fn show_solution(&self) {
        self.solution_is_shown.store(true, Ordering::Relaxed)
    }

    pub fn solution_is_shown(&self) -> bool {
        self.solution_is_shown.load(Ordering::Relaxed)
    }

    pub fn set_results_recorded(&self, recorded: bool) {
        self.results_recorded.store(recorded, Ordering::Relaxed);
    }

    /// Marks the results as recorded and returns whether they already were.
    pub fn mark_results_recorded(&self) -> bool {
        self.results_recorded.swap(true, Ordering::SeqCst)
    }

    pub fn set_picture_state(&self, released: bool, mode: usize) {
        self.picture_released.store(released, Ordering::Relaxed);
        self.picture_mode.store(mode, Ordering::Relaxed);
    }

    pub fn release_picture(&self) {
        self.picture_released.store(true, Ordering::Relaxed);
    }

    pub fn picture_released(&self) -> bool {
        self.picture_released.load(Ordering::Relaxed)
    }

    pub fn set_picture_mode(&self, mode: usize) {
        self.picture_mode.store(mode, Ordering::Relaxed);
    }

    pub fn picture_mode(&self) -> usize {
        self.picture_mode.load(Ordering::Relaxed)
    }

    pub fn answer_is_shown(&self) -> bool {
        self.answer_is_shown.load(Ordering::Relaxed)
    }
}

impl Clone for QuestionState {
    fn clone(&self) -> Self {
        Self {
            answer_is_shown: self.answer_is_shown.load(Ordering::Relaxed).into(),
            solution_is_shown: self.solution_is_shown.load(Ordering::Relaxed).into(),
            results_recorded: self.results_recorded.load(Ordering::Relaxed).into(),
            picture_released: self.picture_released.load(Ordering::Relaxed).into(),
            picture_mode: self.picture_mode.load(Ordering::Relaxed).into(),
        }
    }
}

#[derive(Error, Debug, PartialEq, Clone, Hash)]
pub enum GroupError {
    #[error("group '{0}' not found")]
    NotFound(String),
    #[error("group with name '{0}' already exists")]
    Duplicate(String),
}

impl ServerData {
    pub fn insert_group(&self, name: &str) -> Result<(), GroupError> {
        match self.groups.entry(name.to_owned()) {
            Entry::Vacant(v) => {
                v.insert(GroupData::default());
                Ok(())
            }
            Entry::Occupied(_) => Err(GroupError::Duplicate(name.to_owned())),
        }
    }

    pub fn delete_group(&self, name: &str) {
        if self.groups.remove(name).is_some() {
            info!("group with name '{}' deleted.", name);
        } else {
            info!("group with name '{}' not found.", name);
        }
    }

    pub fn set_group_points(&self, name: impl AsRef<str>, number: f64) -> Result<(), GroupError> {
        let name = name.as_ref();
        let Some(mut g_data) = self.groups.get_mut(name) else {
            return Err(GroupError::NotFound(name.to_owned()))
        };
        g_data.score = number;
        Ok(())
    }

    pub fn add_group_points(&self, name: impl AsRef<str>, number: f64) -> Result<(), GroupError> {
        let name = name.as_ref();
        let Some(mut g_data) = self.groups.get_mut(name) else {
            return Err(GroupError::NotFound(name.to_owned()))
        };
        g_data.score += number;
        Ok(())
    }

    pub fn set_group_answer(
        &self,
        name: impl AsRef<str>,
        answer: Answer,
    ) -> Result<(), GroupError> {
        let name = name.as_ref();
        let Some(mut group_data) = self.groups.get_mut(name) else {
            return Err(GroupError::NotFound(name.to_owned()));
        };
        group_data.answer = Some(answer);
        Ok(())
    }

    pub fn clear_group_answers(&self) {
        self.groups.iter_mut().for_each(|mut group| {
            group.answer = None;
        })
    }

    fn reset_question_state_for(&self, question: Option<&Question>) {
        self.question_state.reset();
        let Some(question) = question else {
            return;
        };

        // A picture that needs a release stays hidden until the admin shows it.
        if question.has_display_picture() && !question.display_picture_requires_release() {
            let mode = if question.display_picture_starts_inline() {
                PICTURE_MODE_INLINE
            } else {
                PICTURE_MODE_HIDDEN
            };
            self.question_state.set_picture_state(true, mode);
        }
    }

    /// Makes `idx` the current question. This is the only place that changes
    /// the current question.
    ///
    /// `questions` must be the content of the locked `self.questions` mutex.
    /// It is passed in (instead of locked here) so that the index, the answers,
    /// the question state and the version change in one step for everybody who
    /// holds that lock while reading them.
    pub fn switch_question(&self, questions: &[Question], idx: usize) {
        self.current_question.store(idx, Ordering::SeqCst);
        self.block_answer.store(false, Ordering::SeqCst);
        self.clear_group_answers();
        self.reset_question_state_for(questions.get(idx));
        self.question_version.fetch_add(1, Ordering::SeqCst);
    }

    /// Awards the points for `question` to the groups, based on their answers.
    pub fn record_results(&self, question: &Question) {
        let mut estimate_list: Vec<(f64, String)> = Vec::new();
        for mut entry in self.groups.iter_mut() {
            let answ = match &entry.answer {
                Some(a) => a,
                None => continue,
            };
            match question {
                Question::Normal {
                    question: _,
                    answers: _,
                    solution,
                    ..
                } => match answ {
                    Answer::Normal(ans) => {
                        if solution == ans {
                            entry.add_score(NORMAL_POINTS);
                        }
                    }
                    _ => continue,
                },
                Question::Estimate {
                    question: _,
                    solution,
                    ..
                } => match answ {
                    Answer::Estimate(ans) => {
                        estimate_list.push(((solution - ans).abs(), entry.key().clone()));
                    }
                    _ => continue,
                },
                Question::Sort {
                    question: _,
                    answers: _,
                    solution,
                    ..
                } => match answ {
                    Answer::Sort(ans) => {
                        if solution == ans {
                            entry.add_score(SORT_POINTS);
                        }
                    }
                    _ => continue,
                },
                Question::Zuordnen {
                    question: _,
                    answers: _,
                    solution,
                    ..
                } => match answ {
                    Answer::Zuordnen(ans) => {
                        let mut selected = [false; 6];
                        for idx in ans {
                            if *idx < 6 {
                                selected[*idx] = true;
                            }
                        }
                        let mut score = 0.0;
                        for i in 0..6 {
                            if solution[i] == selected[i] {
                                score += ASSIGN_POINTS_PER_ITEM;
                            }
                        }
                        if score > 0.0 {
                            entry.add_score(score);
                        }
                    }
                    _ => continue,
                },
            }
        }
        if !estimate_list.is_empty() {
            estimate_list.sort_by(|a, b| a.0.total_cmp(&b.0));
            let rank_points = [ESTIMATE_1_POINTS, ESTIMATE_2_POINTS, ESTIMATE_3_POINTS];
            let mut distinct_rank = 0usize;
            let mut points_for_rank = rank_points[0];

            for i in 0..estimate_list.len() {
                if i > 0 && estimate_list[i].0 != estimate_list[i - 1].0 {
                    distinct_rank += 1;
                    if distinct_rank >= rank_points.len() {
                        break;
                    }
                    points_for_rank = rank_points[distinct_rank];
                }

                // A group that was deleted in the meantime is skipped; the
                // other groups still get their points.
                if let Some(mut group) = self.groups.get_mut(&estimate_list[i].1) {
                    group.add_score(points_for_rank);
                }
            }
        }
    }
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct GroupData {
    pub score: f64,
    pub answer: Option<Answer>,
}

impl GroupData {
    pub fn add_score(&mut self, points: f64) {
        self.score += points;
    }

    pub fn set_score(&mut self, score: f64) {
        self.score = score;
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum PictureTiming {
    SolutionOnly,
    BeforeAndAfter,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum PictureTarget {
    DisplayOnly,
    UiAndDisplay,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PictureMeta {
    pub path: String,
    pub timing: PictureTiming,
    pub target: PictureTarget,
}

impl PictureMeta {
    fn from_directive(directive: &str) -> Result<Self, &'static str> {
        let mut split = directive.trim().splitn(3, ' ');
        match split.next() {
            Some("pic") => {}
            _ => return Err("Invalid picture directive"),
        }

        let flags = split.next().ok_or("Missing picture flags")?;
        let mut flag_chars = flags.chars();
        if flag_chars.next() != Some('-') {
            return Err("Invalid picture flags");
        }

        let timing = match flag_chars.next() {
            Some('s') => PictureTiming::SolutionOnly,
            Some('b') => PictureTiming::BeforeAndAfter,
            _ => return Err("Invalid picture timing flag"),
        };

        let target = match flag_chars.next() {
            Some('d') => PictureTarget::DisplayOnly,
            Some('u') => PictureTarget::UiAndDisplay,
            _ => return Err("Invalid picture target flag"),
        };

        if flag_chars.next().is_some() {
            return Err("Invalid picture flags");
        }

        let raw_path = split.next().ok_or("Missing picture path")?.trim();
        if raw_path.is_empty() {
            return Err("Missing picture path");
        }

        let normalized_path = raw_path.replace('\\', "/");
        let relative_path = normalized_path.as_str();

        if relative_path.is_empty() {
            return Err("Picture path missing file name");
        }

        if relative_path.starts_with('/') {
            return Err("Invalid picture path");
        }

        let components = Path::new(relative_path).components();

        let mut has_file_component = false;
        for component in components {
            match component {
                Component::Normal(_) => has_file_component = true,
                _ => return Err("Invalid picture path"),
            }
        }

        if !has_file_component {
            return Err("Picture path missing file name");
        }

        Ok(Self {
            path: format!("pictures/{relative_path}"),
            timing,
            target,
        })
    }

    fn visible_on_ui(&self) -> bool {
        matches!(self.target, PictureTarget::UiAndDisplay)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum Question {
    Normal {
        question: String,
        answers: [String; 4],
        solution: usize,
        picture: Option<PictureMeta>,
    },
    Estimate {
        question: String,
        solution: f64,
        picture: Option<PictureMeta>,
    },
    Sort {
        question: String,
        answers: [String; 4],
        solution: [usize; 4],
        picture: Option<PictureMeta>,
    },
    Zuordnen {
        question: String,
        answers: [String; 6],
        solution: [bool; 6],
        picture: Option<PictureMeta>,
    },
}

impl Question {
    pub fn normal(question: &str, answers: &[String], solution: usize) -> Self {
        Self::normal_with_picture(question, answers, solution, None)
    }

    pub fn normal_with_picture(
        question: &str,
        answers: &[String],
        solution: usize,
        picture: Option<PictureMeta>,
    ) -> Self {
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
            picture,
        }
    }

    pub fn estimate(question: &str, solution: f64) -> Self {
        Self::estimate_with_picture(question, solution, None)
    }

    pub fn estimate_with_picture(
        question: &str,
        solution: f64,
        picture: Option<PictureMeta>,
    ) -> Self {
        Question::Estimate {
            question: question.to_owned(),
            solution,
            picture,
        }
    }

    pub fn sort(question: &str, answers: &[String], solution: &[usize]) -> Self {
        Self::sort_with_picture(question, answers, solution, None)
    }

    pub fn sort_with_picture(
        question: &str,
        answers: &[String],
        solution: &[usize],
        picture: Option<PictureMeta>,
    ) -> Self {
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
            picture,
        }
    }

    pub fn zuordnen_with_picture(
        question: &str,
        answers: &[String],
        solution: [bool; 6],
        picture: Option<PictureMeta>,
    ) -> Self {
        if answers.len() != 6 {
            panic!("Wrong amount of answers")
        }
        Question::Zuordnen {
            question: question.to_owned(),
            answers: [
                answers[0].clone(),
                answers[1].clone(),
                answers[2].clone(),
                answers[3].clone(),
                answers[4].clone(),
                answers[5].clone(),
            ],
            solution,
            picture,
        }
    }

    fn picture(&self) -> Option<&PictureMeta> {
        match self {
            Question::Normal { picture, .. }
            | Question::Estimate { picture, .. }
            | Question::Sort { picture, .. }
            | Question::Zuordnen { picture, .. } => picture.as_ref(),
        }
    }

    pub fn has_display_picture(&self) -> bool {
        self.picture().is_some()
    }

    pub fn display_picture_requires_release(&self) -> bool {
        let Some(picture) = self.picture() else {
            return false;
        };
        matches!(picture.timing, PictureTiming::SolutionOnly)
    }

    pub fn display_picture_starts_inline(&self) -> bool {
        let Some(picture) = self.picture() else {
            return false;
        };
        matches!(self, Question::Estimate { .. })
            && matches!(picture.timing, PictureTiming::BeforeAndAfter)
    }

    pub fn display_picture_path(&self) -> Option<&str> {
        self.picture().map(|picture| picture.path.as_str())
    }

    pub fn display_image_before(&self) -> Option<&str> {
        let picture = self.picture()?;
        if !matches!(self, Question::Estimate { .. })
            || !matches!(picture.timing, PictureTiming::BeforeAndAfter)
        {
            return None;
        }
        Some(picture.path.as_str())
    }

    pub fn display_image_after(&self) -> Option<&str> {
        let picture = self.picture()?;
        if !matches!(
            picture.timing,
            PictureTiming::SolutionOnly | PictureTiming::BeforeAndAfter
        ) {
            return None;
        }
        Some(picture.path.as_str())
    }

    pub fn ui_image_before(&self) -> Option<&str> {
        let picture = self.picture()?;
        if !picture.visible_on_ui() || !matches!(picture.timing, PictureTiming::BeforeAndAfter) {
            return None;
        }
        Some(picture.path.as_str())
    }

    pub fn ui_image_after(&self) -> Option<&str> {
        let picture = self.picture()?;
        if matches!(picture.timing, PictureTiming::SolutionOnly) {
            return None;
        }
        if !picture.visible_on_ui()
            || !matches!(
                picture.timing,
                PictureTiming::SolutionOnly | PictureTiming::BeforeAndAfter
            )
        {
            return None;
        }
        Some(picture.path.as_str())
    }
}

impl FromStr for Question {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn parse_solution_letter_4(s: &str) -> Result<usize, &'static str> {
            let letter = s.trim().to_ascii_lowercase();
            match letter.as_str() {
                "a" => Ok(0),
                "b" => Ok(1),
                "c" => Ok(2),
                "d" => Ok(3),
                _ => Err("Invalid solution"),
            }
        }

        fn parse_solution_letter_6(letter: char) -> Result<usize, &'static str> {
            match letter.to_ascii_lowercase() {
                'a' => Ok(0),
                'b' => Ok(1),
                'c' => Ok(2),
                'd' => Ok(3),
                'e' => Ok(4),
                'f' => Ok(5),
                _ => Err("Invalid solution"),
            }
        }

        let mut split = s.split('#').collect::<Vec<_>>();
        if split.is_empty() {
            return Err("No question type found");
        }

        let picture = if let Some(last_segment) = split.last() {
            if last_segment.trim().starts_with("pic ") {
                let directive = split.pop().ok_or("Invalid picture directive")?;
                Some(PictureMeta::from_directive(directive.trim())?)
            } else {
                None
            }
        } else {
            None
        };

        let question_type = match split.first() {
            Some(val) if QUESTION_TYPES.contains(val) => *val,
            None | Some(_) => return Err("No question type found"),
        };

        let question = split.get(1).copied().ok_or("No question text found")?;

        match question_type {
            NORMAL_NAME => {
                if split.len() != 7 {
                    return Err("Invalid normal question format");
                }

                let answers = [
                    split[2].to_owned(),
                    split[3].to_owned(),
                    split[4].to_owned(),
                    split[5].to_owned(),
                ];
                let solution = parse_solution_letter_4(split[6])?;

                Ok(Question::normal_with_picture(
                    question,
                    &answers,
                    solution,
                    picture,
                ))
            }
            ESTIMATE_NAME => {
                if split.len() != 3 {
                    return Err("Invalid estimate question format");
                }

                let solution = split[2]
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "Estimate solution not a number")?;
                if !solution.is_finite() {
                    return Err("Estimate solution not a finite number");
                }

                Ok(Question::estimate_with_picture(question, solution, picture))
            }
            SORT_NAME => {
                if split.len() != 10 {
                    return Err("Invalid sort question format");
                }

                let answers = [
                    split[2].to_owned(),
                    split[3].to_owned(),
                    split[4].to_owned(),
                    split[5].to_owned(),
                ];
                let solutions = [
                    parse_solution_letter_4(split[6])?,
                    parse_solution_letter_4(split[7])?,
                    parse_solution_letter_4(split[8])?,
                    parse_solution_letter_4(split[9])?,
                ];

                Ok(Question::sort_with_picture(
                    question,
                    &answers,
                    &solutions,
                    picture,
                ))
            }
            ASSIGN_NAME => {
                if split.len() < 9 {
                    return Err("Invalid zuordnen question format");
                }

                let answers = [
                    split[2].to_owned(),
                    split[3].to_owned(),
                    split[4].to_owned(),
                    split[5].to_owned(),
                    split[6].to_owned(),
                    split[7].to_owned(),
                ];

                let mut solution = [false; 6];
                let mut any_solution = false;
                for segment in &split[8..] {
                    let trimmed = segment.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    for letter in trimmed.chars() {
                        if letter.is_whitespace() || letter == ',' {
                            continue;
                        }
                        let idx = parse_solution_letter_6(letter)?;
                        solution[idx] = true;
                        any_solution = true;
                    }
                }

                if !any_solution {
                    return Err("Missing zuordnen solution");
                }

                Ok(Question::zuordnen_with_picture(
                    question,
                    &answers,
                    solution,
                    picture,
                ))
            }
            _ => Err("?"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Answer {
    Normal(usize),
    Estimate(f64),
    Sort([usize; 4]),
    Zuordnen(Vec<usize>),
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

            // Every item must appear exactly once in an order.
            let mut seen = [false; 4];
            for &number in &numbers {
                if std::mem::replace(&mut seen[number], true) {
                    return Err(format!("\"{letters}\" uses a letter twice"));
                }
            }

            Ok(numbers)
        }

        fn convert_letters_to_set(letters: &str) -> Result<Vec<usize>, String> {
            if letters.trim().is_empty() {
                return Ok(Vec::new());
            }
            let mut present = [false; 6];
            for letter in letters.chars() {
                if letter.is_whitespace() || letter == ',' {
                    continue;
                }
                let idx = match letter.to_ascii_lowercase() {
                    'a' => 0,
                    'b' => 1,
                    'c' => 2,
                    'd' => 3,
                    'e' => 4,
                    'f' => 5,
                    _ => return Err(format!("invalid letter: {letter}")),
                };
                present[idx] = true;
            }

            let mut values = Vec::new();
            for (idx, has_value) in present.iter().enumerate() {
                if *has_value {
                    values.push(idx);
                }
            }
            Ok(values)
        }
        match s {
            NORMAL_NAME => match answer_string.trim().parse::<usize>() {
                Ok(idx) if idx < 4 => Ok(Answer::Normal(idx)),
                _ => Err(format!(
                    "could not parse answer \"{answer_string}\" as an answer index"
                )),
            },
            ESTIMATE_NAME => match answer_string.trim().parse::<f64>() {
                Ok(value) if value.is_finite() => Ok(Answer::Estimate(value)),
                _ => Err(format!(
                    "could not parse answer \"{answer_string}\" as a number"
                )),
            },
            SORT_NAME => convert_letters_to_numbers(answer_string).map(Answer::Sort),
            ASSIGN_NAME => convert_letters_to_set(answer_string).map(Answer::Zuordnen),
            _ => Err(format!("invalid answer type: \"{s}\"")),
        }
    }

    /// Whether this is an answer to a question of the type of `question`.
    pub fn matches_question(&self, question: &Question) -> bool {
        matches!(
            (self, question),
            (Answer::Normal(_), Question::Normal { .. })
                | (Answer::Estimate(_), Question::Estimate { .. })
                | (Answer::Sort(_), Question::Sort { .. })
                | (Answer::Zuordnen(_), Question::Zuordnen { .. })
        )
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
    ToggleLeaderboard,
    ShowPictureInline,
    ShowPictureTop,
    ShowPictureFullscreen,
    HidePicture,
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
                ToggleLeaderboard => "toggle_leaderboard",
                ShowPictureInline => "show_picture_inline",
                ShowPictureTop => "show_picture_top",
                ShowPictureFullscreen => "show_picture_fullscreen",
                HidePicture => "hide_picture",
            }
        )
    }
}

/// Sends `event` to every connected client. Call this after the state the
/// event announces has been changed, because clients react by fetching it.
pub fn send_event(server_data: &ServerData, event: UpdateEvent) {
    info!("sending event with text: {event}");
    // An error only means that nobody is listening at the moment.
    let _ = server_data.events.send(event);
}
