use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::process::exit;
use clover::{Clover, Program, State};
use clover_std::clover_std_inject_to;
use clap::{Arg, Parser};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    major_version: u8,
    minor_version: u8,
    patch_version: u8,
    compile: Option<bool>,
    output_filename: Option<String>,
    filename: String,
}

impl Config {
    /// Validates the configuration fields to ensure they meet expected criteria.
    fn validate(&self) -> Result<(), ConfigError> {
        if self.filename.trim().is_empty() {
            return Err(ConfigError::InvalidField("filename cannot be empty".into()));
        }

        if let Some(ref output) = self.output_filename {
            if output.trim().is_empty() {
                return Err(ConfigError::InvalidField("output_filename cannot be empty".into()));
            }
            // Additional validations for output_filename can be added here
        }

        // Add more validations as necessary
        Ok(())
    }
}

/// Custom error type for configuration validation
#[derive(Debug)]
enum ConfigError {
    InvalidField(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::InvalidField(msg) => write!(f, "Invalid field: {}", msg),
        }
    }
}

impl Error for ConfigError {}

fn load_config(filename: &str) -> Result<Config, Box<dyn Error>> {
    let mut file = File::open(filename)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let config: Config = serde_json::from_str(&contents)?;
    
    // Validate the configuration after deserialization
    config.validate()?;
    
    Ok(config)
}

fn setup() -> Result<(), Box<dyn Error>> {
    println!("Setting up...");

    // Create config file if not exist
    let config = Config {
        major_version: clover::version::MAJOR,
        minor_version: clover::version::MINOR,
        patch_version: clover::version::PATCH,
        compile: None,
        output_filename: None,
        filename: String::from("build.pie"),
    };
    
    // Validate the configuration before writing
    config.validate()?;
    
    let json = serde_json::to_string_pretty(&config)?;
    let mut file = File::create("config.json")?;
    file.write_all(json.as_bytes())?;

    Ok(())
}

#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
    /// Compile input file
    #[clap(short, long, action)]
    compile: bool,

    /// Specify the output filename when compile
    #[clap(short, long = "output", value_parser)]
    output_filename: Option<String>,

    /// Source filename to run/compile
    #[clap(value_parser)]
    pub filename: String,
}


/// Entry point of the program
fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // Setup if not exist
    let config_file = std::path::Path::new("config.json");
    if !config_file.exists() {
        setup()?
    } else {
        let current_version = format!("{}.{}.{}", clover::version::MAJOR, clover::version::MINOR, clover::version::PATCH);
        let config = load_config("config.json")?;
        let stored_version = format!("{}.{}.{}", config.major_version, config.minor_version, config.patch_version);
        if stored_version != current_version {
            println!("Config is outdated. Rebuilding...");
            setup()?;
        }
    }

    // Load config if specified
    let config = if let Some(config_file) = args.filename.strip_suffix(".json") {
        Some(load_config(config_file)?)
    } else {
       // Load config if not specified
       Some(load_config("config.json")?)
    };

    let pie = Clover::new();

    let filename: String = args.filename.clone();

    let program = if filename.ends_with(".lucky") {
        if args.compile {
            // Can not compile a pie file
            println!("Can not compile pie file.");
            exit(-1);
        }

        let mut file = File::open(filename)?;
        Program::deserialize(&mut file, true)?
    } else {
        pie.compile_file(filename.as_str())?
    };

    if args.compile {
        let output_filename: String = args.output_filename.unwrap_or(if args.filename.ends_with("luck") { args.filename + "y" } else { args.filename + ".lucky" });

        let mut file = File::create(output_filename)?;

        program.serialize(&mut file, true)?;

    } else {
        let mut state = program.into();

        clover_std_inject_to(&mut state);

        state.execute()?;
    }

    Ok(())
}
