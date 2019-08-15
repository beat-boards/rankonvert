extern crate beatmap_parser;
extern crate csv;
extern crate ctrlc;
extern crate serde;
extern crate serde_json;
extern crate threadpool;

use beatmap_parser::difficulty::difficulty::note::NoteType;
use beatmap_parser::info::info::difficulty_beatmap_set::difficulty_beatmap::DifficultyRank;
use beatmap_parser::info::info::difficulty_beatmap_set::BeatmapCharacteristic;
use beatmap_parser::Beatmap;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::process;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;

#[derive(Debug, Deserialize, Serialize)]
struct RatedMap {
    download: String,
    difficulty: String,
    rating: f64,
}

#[derive(Debug, Serialize)]
struct RatedMapData {
    rating: f64,
    is_easy: u8,
    is_normal: u8,
    is_hard: u8,
    is_expert: u8,
    is_expert_plus: u8,
    length: f64,
    bpm: f64,
    note_jump_speed: f64,
    note_count: u32,
    bomb_count: u32,
    notes_per_second: f64,
    obstacle_count: u32,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 || args.len() > 5 {
        panic!("Invalid arguments");
    }

    let input_contents = fs::read_to_string(&args[1]).expect("Invalid input file");
    let rated_maps: Vec<RatedMap> =
        serde_json::from_str(&input_contents).expect("Invalid input contents");

    let pool_size: usize = (&args[3]).parse().expect("Invalid pool size");
    let pool = ThreadPool::new(pool_size);

    let writer = csv::Writer::from_path(&args[2]).expect("Invalid output file");
    let writer = Arc::new(Mutex::new(writer));

    let (tx, rx) = mpsc::channel();
    let map_count = rated_maps.len();

    let ctrlc_writer = writer.clone();
    ctrlc::set_handler(move || {
        let mut w = ctrlc_writer.lock().expect("Can't lock writer");
        (*w).flush().expect("Can't close output");

        process::exit(0);
    })
    .expect("Can't set Ctrl-C handler");

    for rated_map in rated_maps {
        let writer = writer.clone();
        let tx = tx.clone();
        pool.execute(move || {
            println!("Parsing info for {:#?}", &rated_map);

            let beatmap = Beatmap::from_beatsaver_url(&rated_map.download)
                .expect(&format!("Can't parse beatmap {}", &rated_map.download));

            let mut is_easy: u8 = 0;
            let mut is_normal: u8 = 0;
            let mut is_hard: u8 = 0;
            let mut is_expert: u8 = 0;
            let mut is_expert_plus: u8 = 0;

            let difficulty_rank = match &rated_map.difficulty[..] {
                "Easy" => {
                    is_easy = 1;
                    DifficultyRank::Easy
                }
                "Normal" => {
                    is_normal = 1;
                    DifficultyRank::Normal
                }
                "Hard" => {
                    is_hard = 1;
                    DifficultyRank::Hard
                }
                "Expert" => {
                    is_expert = 1;
                    DifficultyRank::Expert
                }
                "ExpertPlus" => {
                    is_expert_plus = 1;
                    DifficultyRank::ExpertPlus
                }
                _ => {
                    panic!("Invalid input difficulty");
                }
            };

            let difficulty = &beatmap
                .difficulties
                .get(&BeatmapCharacteristic::Standard)
                .expect("Can't find standard maps")
                .get(&difficulty_rank)
                .expect("Can't find specified difficulty");

            let rating = &rated_map.rating;
            let length = &beatmap.length;
            let bpm = &beatmap.info.beats_per_minute;
            let note_jump_speed = &beatmap
                .info
                .difficulty_beatmap_sets
                .iter()
                .find(|&x| x.beatmap_characteristic_name == BeatmapCharacteristic::Standard)
                .expect("Can't find standard maps")
                .difficulty_beatmaps
                .iter()
                .find(|&x| x.difficulty_rank == difficulty_rank)
                .expect("Can't find specified difficulty")
                .note_jump_movement_speed;
            let (note_count, bomb_count) = {
                let mut i: u32 = 0;
                let mut j: u32 = 0;
                for note in &difficulty.notes {
                    if note.note_type != NoteType::Bomb {
                        i += 1;
                    } else {
                        j += 1;
                    }
                }
                (i, j)
            };
            let notes_per_second = note_count as f64 / length.clone();
            let obstacle_count = &difficulty.obstacles.len();

            let rated_map_data = RatedMapData {
                rating: rating.clone(),
                is_easy,
                is_normal,
                is_hard,
                is_expert,
                is_expert_plus,
                length: length.clone(),
                bpm: bpm.clone(),
                note_jump_speed: note_jump_speed.clone(),
                note_count,
                bomb_count,
                notes_per_second,
                obstacle_count: obstacle_count.clone() as u32,
            };

            println!(
                "Parsed info for {}: {:#?}",
                &rated_map.download, &rated_map_data
            );

            let mut w = writer.lock().expect("Can't lock writer");
            (*w).serialize(&rated_map_data)
                .expect("Can't write to output");
            tx.send(true).expect("Can't send message to mpsc channel");
        });
    }

    for _ in 0..map_count {
        rx.recv().expect("Can't read message from mpsc channel");
    }
    let mut w = writer.lock().expect("Can't lock writer");
    (*w).flush().expect("Can't close output");
}
