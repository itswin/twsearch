use std::{collections::HashMap, mem, time::Instant, vec};

use thousands::Separable;

use cubing::alg::Move;

use crate::{
    gods_algorithm::factor_number::factor_number, ConversionError, PackedKPattern, PackedKPuzzle,
    PackedKTransformation,
};

type SearchDepth = usize;

use indicatif::{MultiProgress, ProgressBar, ProgressFinish, ProgressStyle};

use super::bulk_queue::BulkQueue;

pub struct GodsAlgorithmTable {
    completed: bool, // "completed" instead of "complete" to make an unambiguous adjective
    pattern_to_depth: HashMap<PackedKPattern, /* depth */ SearchDepth>,
}

impl GodsAlgorithmTable {
    pub fn new() -> Self {
        Self {
            completed: false,
            pattern_to_depth: HashMap::new(),
        }
    }
}

impl Default for GodsAlgorithmTable {
    fn default() -> Self {
        Self::new()
    }
}

struct CachedMoveInfo {
    _move: Move,
    _transformation: PackedKTransformation,
    inverse_transformation: PackedKTransformation,
}

impl CachedMoveInfo {
    pub fn try_new(packed_kpuzzle: &PackedKPuzzle, r#move: Move) -> Result<Self, ConversionError> {
        Ok(Self {
            _move: r#move.clone(),
            _transformation: packed_kpuzzle.transformation_from_move(&r#move)?,
            inverse_transformation: packed_kpuzzle.transformation_from_move(&(r#move).invert())?, // TODO: invert the regular transformation directly.
        })
    }
}

pub struct GodsAlgorithmSearch {
    // params
    packed_kpuzzle: PackedKPuzzle,
    start_pattern: Option<PackedKPattern>,
    cached_move_info_list: Vec<CachedMoveInfo>,

    // state
    table: GodsAlgorithmTable,
    bulk_queues: Vec<BulkQueue<PackedKPattern>>, // TODO: `HashMap` instead of `Vec` for the other layer for sparse rep?

    multi_progress_bar: MultiProgress,
}

macro_rules! format_num {
    ($n:expr) => {
        $n.separate_with_underscores()
    };
}

impl GodsAlgorithmSearch {
    pub fn try_new(
        packed_kpuzzle: PackedKPuzzle,
        start_pattern: Option<PackedKPattern>,
        move_list: Vec<Move>,
    ) -> Result<Self, String> {
        let move_list: Result<Vec<CachedMoveInfo>, ConversionError> = move_list
            .into_iter()
            .map(|r#move| CachedMoveInfo::try_new(&packed_kpuzzle, r#move))
            .collect();

        let move_list = move_list.map_err(|e| e.to_string())?;
        let depth_to_patterns = vec![];
        Ok(Self {
            packed_kpuzzle,
            start_pattern,
            cached_move_info_list: move_list,
            table: GodsAlgorithmTable {
                completed: false,
                pattern_to_depth: HashMap::new(),
            },
            bulk_queues: depth_to_patterns,
            multi_progress_bar: MultiProgress::new(),
        })
    }

    pub fn fill(&mut self) {
        let start_pattern = match &self.start_pattern {
            Some(start_pattern) => start_pattern.clone(),
            None => self.packed_kpuzzle.default_pattern(),
        };
        self.table.pattern_to_depth.insert(start_pattern.clone(), 0);
        self.bulk_queues.push(BulkQueue::new(Some(start_pattern)));

        let mut current_depth = 0;
        let mut num_patterns_total = 1;

        let start_time = Instant::now();
        while !self.table.completed {
            let last_depth_patterns: BulkQueue<PackedKPattern> = mem::replace(
                &mut self.bulk_queues[current_depth],
                BulkQueue::bogus_new(), // TODO: change the field to avoid the need for this?
            );
            let num_last_depth_patterns = last_depth_patterns.size();

            current_depth += 1;

            let progress_bar = ProgressBar::new(num_last_depth_patterns.try_into().unwrap());
            let progress_bar = self.multi_progress_bar.insert_from_back(0, progress_bar);
            let progress_bar = progress_bar.with_finish(ProgressFinish::AndLeave);
            // TODO share the progress bar style?
            let progress_bar_style = ProgressStyle::with_template(
                "{prefix:3} {bar:12.cyan/blue} {elapsed:.2} {wide_msg}",
            )
            .expect("Could not construct progress bar.");
            // .progress_chars("=> ");
            progress_bar.set_style(progress_bar_style);
            progress_bar.set_prefix(current_depth.to_string());

            let num_to_test_at_current_depth: usize =
                num_last_depth_patterns * self.cached_move_info_list.len();
            let mut num_tested_at_current_depth = 0;
            let mut patterns_at_current_depth = BulkQueue::new(None);
            for sublist in last_depth_patterns.sublists_for_iterator() {
                for pattern in sublist {
                    for move_info in &self.cached_move_info_list {
                        num_tested_at_current_depth += 1;
                        let new_pattern =
                            pattern.apply_transformation(&move_info.inverse_transformation);
                        if self.table.pattern_to_depth.get(&new_pattern).is_some() {
                            continue;
                        }

                        patterns_at_current_depth.push(new_pattern.clone());
                        self.table
                            .pattern_to_depth
                            .insert(new_pattern, current_depth);

                        if num_tested_at_current_depth % 1000 == 0 {
                            progress_bar
                                .set_length(num_to_test_at_current_depth.try_into().unwrap());
                            progress_bar.set_position(num_tested_at_current_depth);
                            progress_bar.set_message(format!(
                                "{} patterns ({} cumulative) — {} remaining candidates",
                                format_num!(patterns_at_current_depth.size()),
                                format_num!(num_patterns_total + patterns_at_current_depth.size()), // TODO: increment before
                                format_num!(
                                    num_to_test_at_current_depth
                                        - std::convert::TryInto::<usize>::try_into(
                                            num_tested_at_current_depth
                                        )
                                        .unwrap()
                                )
                            ))
                            // print!(".");
                            // stdout().flush().unwrap();
                            // println!(
                            //     "Found {} pattern{} at depth {} so far.",
                            //     num_patterns_at_current_depth,
                            //     if num_patterns_at_current_depth == 1 {
                            //         ""
                            //     } else {
                            //         "s"
                            //     },
                            //     current_depth
                            // );
                        }
                    }
                }
            }
            let num_patterns_at_current_depth = patterns_at_current_depth.size();
            num_patterns_total += num_patterns_at_current_depth;
            {
                progress_bar.set_length(patterns_at_current_depth.size().try_into().unwrap());
                progress_bar.set_position(patterns_at_current_depth.size().try_into().unwrap());
                progress_bar.set_message(format!(
                    "{} patterns ({} cumulative)",
                    format_num!(num_patterns_at_current_depth),
                    format_num!(num_patterns_total)
                ))
            }
            self.bulk_queues.push(patterns_at_current_depth);
            // println!();

            if num_patterns_at_current_depth == 0 {
                progress_bar.finish_and_clear();
                self.table.completed = true;
                // let progress_bar_style = ProgressStyle::with_template(
                //     "{prefix:3} {bar:12.red/blue} {elapsed:.2} {wide_msg}",
                // )
                // .expect("Could not construct progress bar.");
                // // .progress_chars("=> ");
                // progress_bar.set_style(progress_bar_style);
            } else {
                progress_bar.finish();
            }
        }
        let max_depth = current_depth - 1;
        println!();
        println!();
        println!(
            "Found {} ({}) pattern{}.\nMaximum depth: {} moves\nTotal time elapsed: {:?}",
            format_num!(num_patterns_total),
            factor_number(num_patterns_total.try_into().unwrap()),
            if num_patterns_total == 1 { "" } else { "s" },
            max_depth,
            Instant::now() - start_time
        );
    }
}
