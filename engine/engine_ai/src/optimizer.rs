use std::{
    collections::VecDeque,
    fs::{self, File},
    io::{self, BufRead, BufReader, BufWriter, Write},
    sync::{mpsc::channel, Arc, Mutex},
    thread,
};

use rand::{distr::weighted::WeightedIndex, rng, RngExt};

use crate::{battle::do_battle, model::Model, mutate::Mutateable};

const POPULATION_SIZE: usize = 20;
const ELITE_COUNT: usize = 5;
const BATTLES_PER_PAIR: usize = 6;
const WORKERS: usize = 12;
const POP_FILE: &str = "pop.json";
const BEST_DIR: &str = "best";

pub fn run_optimizer(generations: usize) {
    let mut population = load_population(POP_FILE).unwrap_or_else(|e| {
        eprintln!("Failed to load pop.json: {e}. Generating new population.");
        new_population()
    });

    for _ in 0..generations {
        let results = run_generation(&population);
        let next_population = build_next_population(&population, &results);

        if let Err(e) = save_population(POP_FILE, &next_population) {
            eprintln!("Failed to save pop.json: {e}");
        }

        if let Err(e) = save_best_member(&next_population) {
            eprintln!("Failed to save best member: {e}");
        }

        population = next_population;
    }
}

#[derive(Clone, Debug)]
struct Population {
    generation: usize,
    members: Vec<Model>,
}

fn new_population() -> Population {
    let mut members = Vec::with_capacity(POPULATION_SIZE);
    members.push(Model::default());
    for num in 0..(POPULATION_SIZE - 1) {
        members.push(Model::generate(format!("Gen 0 #{}", num)));
    }

    Population {
        generation: 0,
        members,
    }
}

fn run_generation(population: &Population) -> Vec<(usize, i32)> {
    let mut matchups = VecDeque::new();
    for i in 0..population.members.len() {
        for j in 0..population.members.len() {
            if i == j {
                continue;
            }
            for _ in 0..BATTLES_PER_PAIR {
                matchups.push_back((
                    i,
                    population.members[i].clone(),
                    j,
                    population.members[j].clone(),
                ));
            }
        }
    }

    let total_matches = matchups.len();
    let matchups = Arc::new(Mutex::new((true, matchups)));
    let (send, recv) = channel::<Option<usize>>();

    for _ in 0..WORKERS {
        let matchups = matchups.clone();
        let send = send.clone();
        thread::spawn(move || loop {
            let (p1_idx, p1, p2_idx, p2) = {
                let (active, ref mut queue) = *matchups.lock().unwrap();
                if !active {
                    break;
                }
                match queue.pop_front() {
                    Some(v) => v,
                    None => continue,
                }
            };

            let winner = match do_battle(&p1, &p2) {
                1 => Some(p1_idx),
                -1 => Some(p2_idx),
                _ => None,
            };

            let _ = send.send(winner);
        });
    }

    let mut results = (0..population.members.len()).map(|i| (i, 0)).collect::<Vec<_>>();
    for i in 0..total_matches {
        if let Ok(Some(winner)) = recv.recv() {
            results[winner].1 += 1;
        }

        // if (i + 1) % 80 == 0 {
            println!("Completed game {} of {}", i + 1, total_matches);
        // }
    }

    matchups.lock().unwrap().0 = false;

    results.sort_by_key(|(_, wins)| -wins);
    println!("Gen {} Results:", population.generation);
    for &(idx, wins) in &results {
        println!("{}: {} wins", population.members[idx].name(), wins);
    }
    println!();

    results
}

fn build_next_population(population: &Population, results: &[(usize, i32)]) -> Population {
    let mut new_population = Population {
        generation: population.generation + 1,
        members: Vec::with_capacity(population.members.len()),
    };

    for &(idx, _) in results {
        new_population.members.push(population.members[idx].clone());
    }

    let weighted = WeightedIndex::new(results.iter().map(|&(_, v)| v * v + 1))
        .expect("invalid weights for selection");

    for i in ELITE_COUNT..new_population.members.len() {
        let mut p1 = rng().sample(&weighted);
        let mut p2 = p1;
        while p1 == p2 {
            p2 = rng().sample(&weighted);
        }

        new_population.members[i] = Model::crossover(
            &population.members[results[p1].0],
            &population.members[results[p2].0],
            format!("Gen {} #{}", new_population.generation, i - ELITE_COUNT),
        );
    }

    new_population
}

fn save_best_member(population: &Population) -> io::Result<()> {
    fs::create_dir_all(BEST_DIR)?;
    let path = format!("{}/{}.json", BEST_DIR, population.generation);
    let mut file = BufWriter::new(File::create(path)?);
    write_population_header(&mut file, population.generation, 1)?;
    write_model(&mut file, &population.members[0])?;
    Ok(())
}

fn save_population(path: &str, population: &Population) -> io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_population_header(&mut file, population.generation, population.members.len())?;
    for member in &population.members {
        write_model(&mut file, member)?;
    }
    Ok(())
}

fn write_population_header(
    mut writer: impl Write,
    generation: usize,
    members: usize,
) -> io::Result<()> {
    writeln!(writer, "generation={}", generation)?;
    writeln!(writer, "members={}", members)?;
    Ok(())
}

fn write_model(mut writer: impl Write, model: &Model) -> io::Result<()> {
    writeln!(writer, "member_begin")?;
    writeln!(writer, "name={}", encode_name(&model.name))?;
    write_i32(&mut writer, "back_to_back", model.back_to_back)?;
    write_i32(&mut writer, "bumpiness", model.bumpiness)?;
    write_i32(&mut writer, "bumpiness_sq", model.bumpiness_sq)?;
    write_i32(&mut writer, "row_transitions", model.row_transitions)?;
    write_i32(&mut writer, "height", model.height)?;
    write_i32(&mut writer, "top_half", model.top_half)?;
    write_i32(&mut writer, "top_quarter", model.top_quarter)?;
    write_i32(&mut writer, "jeopardy", model.jeopardy)?;
    write_i32(&mut writer, "cavity_cells", model.cavity_cells)?;
    write_i32(&mut writer, "cavity_cells_sq", model.cavity_cells_sq)?;
    write_i32(&mut writer, "overhang_cells", model.overhang_cells)?;
    write_i32(&mut writer, "overhang_cells_sq", model.overhang_cells_sq)?;
    write_i32(&mut writer, "covered_cells", model.covered_cells)?;
    write_i32(&mut writer, "covered_cells_sq", model.covered_cells_sq)?;
    write_i32(&mut writer, "well_depth", model.well_depth)?;
    write_i32(&mut writer, "max_well_depth", model.max_well_depth)?;
    write_i32_array(&mut writer, "well_column", &model.well_column)?;
    write_i32(&mut writer, "b2b_clear", model.b2b_clear)?;
    write_i32_array(&mut writer, "clear", &model.clear)?;
    write_i32_array(&mut writer, "spin", &model.spin)?;
    write_i32_array(&mut writer, "spin_mini", &model.spin_mini)?;
    write_i32(&mut writer, "perfect_clear", model.perfect_clear)?;
    write_i32(&mut writer, "combo_garbage", model.combo_garbage)?;
    write_i32_array(&mut writer, "waste", &model.waste)?;
    write_i32(&mut writer, "incoming_garbage", model.incoming_garbage)?;
    write_i32(&mut writer, "outgoing_garbage", model.outgoing_garbage)?;
    write_i32(&mut writer, "b2b_cap", model.b2b_cap)?;
    write_i32(&mut writer, "broke_surge", model.broke_surge)?;
    writeln!(writer, "member_end")?;
    Ok(())
}

fn write_i32(mut writer: impl Write, key: &str, value: i32) -> io::Result<()> {
    writeln!(writer, "{}={}", key, value)
}

fn write_i32_array<const N: usize>(
    mut writer: impl Write,
    key: &str,
    value: &[i32; N],
) -> io::Result<()> {
    let values = value
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(",");
    writeln!(writer, "{}={}", key, values)
}

fn load_population(path: &str) -> io::Result<Population> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file).lines();

    let generation = read_usize_line(reader.next(), "generation")?;
    let members = read_usize_line(reader.next(), "members")?;

    let mut parsed = Vec::with_capacity(members);
    for _ in 0..members {
        let line = read_line(&mut reader)?;
        if line != "member_begin" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "missing member_begin",
            ));
        }
        parsed.push(read_model(&mut reader)?);
    }

    Ok(Population {
        generation,
        members: parsed,
    })
}

fn read_model(reader: &mut impl Iterator<Item = io::Result<String>>) -> io::Result<Model> {
    let name = decode_name(read_string(reader, "name")?);
    let back_to_back = read_i32(reader, "back_to_back")?;
    let bumpiness = read_i32(reader, "bumpiness")?;
    let bumpiness_sq = read_i32(reader, "bumpiness_sq")?;
    let row_transitions = read_i32(reader, "row_transitions")?;
    let height = read_i32(reader, "height")?;
    let top_half = read_i32(reader, "top_half")?;
    let top_quarter = read_i32(reader, "top_quarter")?;
    let jeopardy = read_i32(reader, "jeopardy")?;
    let cavity_cells = read_i32(reader, "cavity_cells")?;
    let cavity_cells_sq = read_i32(reader, "cavity_cells_sq")?;
    let overhang_cells = read_i32(reader, "overhang_cells")?;
    let overhang_cells_sq = read_i32(reader, "overhang_cells_sq")?;
    let covered_cells = read_i32(reader, "covered_cells")?;
    let covered_cells_sq = read_i32(reader, "covered_cells_sq")?;
    let well_depth = read_i32(reader, "well_depth")?;
    let max_well_depth = read_i32(reader, "max_well_depth")?;
    let well_column = read_i32_array(reader, "well_column")?;
    let b2b_clear = read_i32(reader, "b2b_clear")?;
    let clear = read_i32_array(reader, "clear")?;
    let spin = read_i32_array(reader, "spin")?;
    let spin_mini = read_i32_array(reader, "spin_mini")?;
    let perfect_clear = read_i32(reader, "perfect_clear")?;
    let combo_garbage = read_i32(reader, "combo_garbage")?;
    let waste = read_i32_array(reader, "waste")?;
    let incoming_garbage = read_i32(reader, "incoming_garbage")?;
    let outgoing_garbage = read_i32(reader, "outgoing_garbage")?;
    let b2b_cap = read_i32(reader, "b2b_cap")?;
    let broke_surge = read_i32(reader, "broke_surge")?;

    let line = read_line(reader)?;
    if line != "member_end" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "missing member_end",
        ));
    }

    Ok(Model {
        back_to_back,
        bumpiness,
        bumpiness_sq,
        row_transitions,
        height,
        top_half,
        top_quarter,
        jeopardy,
        cavity_cells,
        cavity_cells_sq,
        overhang_cells,
        overhang_cells_sq,
        covered_cells,
        covered_cells_sq,
        well_depth,
        max_well_depth,
        well_column,
        b2b_clear,
        clear,
        spin,
        spin_mini,
        perfect_clear,
        combo_garbage,
        waste,
        incoming_garbage,
        outgoing_garbage,
        b2b_cap,
        broke_surge,
        name,
    })
}

fn read_usize_line(line: Option<io::Result<String>>, key: &str) -> io::Result<usize> {
    let line = line.ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "missing line"))??;
    parse_key_value(&line, key)
}

fn read_string(reader: &mut impl Iterator<Item = io::Result<String>>, key: &str) -> io::Result<String> {
    let line = read_line(reader)?;
    parse_key_value(&line, key)
}

fn read_i32(reader: &mut impl Iterator<Item = io::Result<String>>, key: &str) -> io::Result<i32> {
    let line = read_line(reader)?;
    parse_key_value(&line, key)
}

fn read_i32_array<const N: usize>(
    reader: &mut impl Iterator<Item = io::Result<String>>,
    key: &str,
) -> io::Result<[i32; N]> {
    let line = read_line(reader)?;
    let values: String = parse_key_value(&line, key)?;
    let parts = values.split(',').collect::<Vec<_>>();
    if parts.len() != N {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "array length mismatch",
        ));
    }
    let mut out = [0; N];
    for (idx, part) in parts.into_iter().enumerate() {
        out[idx] = part.parse().map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "invalid integer")
        })?;
    }
    Ok(out)
}

fn read_line(reader: &mut impl Iterator<Item = io::Result<String>>) -> io::Result<String> {
    reader
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "missing line"))?
}

fn parse_key_value<T: std::str::FromStr>(line: &str, key: &str) -> io::Result<T> {
    let (k, v) = line
        .split_once('=')
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing '='"))?;
    if k != key {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("expected key {key}"),
        ));
    }
    v.parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "parse error"))
}

fn encode_name(name: &str) -> String {
    name.bytes()
        .flat_map(|b| match b {
            b'%' => "%25".bytes().collect::<Vec<_>>(),
            b'\n' => "%0A".bytes().collect::<Vec<_>>(),
            b'=' => "%3D".bytes().collect::<Vec<_>>(),
            _ => vec![b],
        })
        .map(|b| b as char)
        .collect()
}

fn decode_name(encoded: String) -> String {
    let mut out = String::new();
    let mut chars = encoded.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let a = chars.next();
            let b = chars.next();
            if let (Some(a), Some(b)) = (a, b) {
                let code = format!("{a}{b}");
                match code.as_str() {
                    "25" => out.push('%'),
                    "0A" => out.push('\n'),
                    "3D" => out.push('='),
                    _ => {
                        out.push('%');
                        out.push(a);
                        out.push(b);
                    }
                }
            } else {
                out.push('%');
                if let Some(a) = a {
                    out.push(a);
                }
                if let Some(b) = b {
                    out.push(b);
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}
