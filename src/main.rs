use std::path::{PathBuf, Path};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use serde_derive::{Serialize, Deserialize};

type Database = HashMap<String, i32>;

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    command: String
}

impl Default for Config {
    fn default() -> Self {
        Config { command: "j".to_string() }
    }
}

fn data_path() -> Option<PathBuf> {
    if let Some(parent) = dirs::data_dir() {
        let dir = parent.join("jump_rust");
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("{}", e);
            return None;
        }
        Some(dir)
    } else {
        None
    }
}

fn write_config(path: &Path, config: &Config) -> std::io::Result<()> {
    let mut f = BufWriter::new(File::create(path)?);
    f.write_all(&toml::to_vec(config).unwrap()[..])
}

fn load_config(path: &Path) -> Config {
    let data = std::fs::read(path).unwrap_or_default();
    if data.is_empty() {
        let default = Config::default();
        write_config(path, &default).unwrap_or_else(|e| eprintln!("Failed to write config: {}", e));
        return default;
    }
    toml::from_slice(&data[..]).unwrap()
}

fn write_database(path: &Path, db: &Database) -> std::io::Result<()> {
    let f = BufWriter::new(File::create(path)?);
    bincode::serialize_into(f, &db).expect("Failed to serialize");
    Ok(())
}

fn load_database(path: &Path) -> std::io::Result<Database> {
    let f = BufReader::new(File::open(path)?);
    let decoded: Database = bincode::deserialize_from(f).unwrap();
    Ok(decoded)
}

fn cd(re_str: &str, db: &mut Database) -> Option<String> {
    if Path::new(re_str).exists() || re_str == "/" {
        return Some(re_str.to_owned());
    }
    let result = re_str.to_owned();
    let mut matches = Vec::new();
    match regex::Regex::new(re_str).unwrap() {
        re => {
            for (entry, _score) in db {
                if re.is_match(entry) {
                    matches.push(entry.to_owned());
                }
            }
        }
    }
    matches.sort();
    Some(matches.first().unwrap_or(&result).to_owned())
}

fn ch_dir(dir: &Path, db: &mut Database, db_path: &Path) -> std::io::Result<()> {
    let last = *db.get(dir.to_str().unwrap()).unwrap_or(&0);
    db.insert(dir.to_str().unwrap().to_owned(), last + 1);
    write_database(db_path, db)
}

fn main() {
    let config_path_buf = data_path().unwrap().join("config.toml");
    let config_path = config_path_buf.as_path();
    let mut config = load_config(config_path);
    let db_path_buf = data_path().unwrap().join("database.jdb");
    let db_path = db_path_buf.as_path();
    let mut db = load_database(db_path).unwrap_or_else(|_| {
        let d = HashMap::new();
        write_database(db_path, &d).expect("Failed to write database");
        d
    });

    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).unwrap_or_else(|| {
        eprintln!("Missing command");
        std::process::exit(1);
    });

    match cmd.as_str() {
        "cd" => {
            println!("{}", cd(args.get(2).unwrap_or(&std::env::var("OLDPWD").unwrap_or("".to_owned())), &mut db).unwrap());
        }
        "chdir" => ch_dir(Path::new(std::env::current_dir().unwrap_or_else(|e| {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }).as_path()), &mut db, db_path).unwrap_or_else(|e| eprintln!("{}", e)),
        "shell" => println!("__jump_prompt_command() {{
    local status=$?
    jump chdir && return $status
}}

{}() {{
    local dir=\"$(jump cd \"$@\")\"
    test -d \"$dir\" && cd \"$dir\" || echo 'directory not found'
}}

[[ \"$PROMPT_COMMAND\" =~ __jump_prompt_command ]] || {{
    PROMPT_COMMAND=\"__jump_prompt_command;$PROMPT_COMMAND\"
}}", config.command),
        "config" => {
            if args.len() == 2 {
                println!("{:#?}", config);
                return;
            }

            let parts: Vec<&str> = args.get(2).unwrap().split('=').collect();

            if *parts.first().unwrap() == "command" {
                config.command = (*parts.last().unwrap()).into();
            }

            write_config(config_path, &config).unwrap_or_else(|e| eprintln!("Failed to write config: {}", e));
        }
        "reset" => {
            db = HashMap::new();
            write_database(db_path, &mut db).unwrap_or_else(|e| {
                eprintln!("Failed to write to database: {}", e);
                std::process::exit(1);
            })
        }
        "print" => println!("{:#?}", db),
        _ => {
            eprintln!("Invalid command");
        }
    }
}
