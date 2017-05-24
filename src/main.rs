extern crate getopts;

use getopts::Options;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
enum BuildSystem {
    Make,
    Ninja,
}

impl BuildSystem {
    fn as_cmake_arg(&self) -> &'static str {
        match *self {
            BuildSystem::Make => "-GCodeBlocks - Unix Makefiles",
            BuildSystem::Ninja => "-GCodeBlocks - Ninja",
        }
    }
}

enum Compiler {
    Gcc,
    Clang,
}

use std::fmt::{Display, Formatter};

impl Display for Compiler {
    fn fmt(&self, fmtr: &mut Formatter) -> Result<(), std::fmt::Error> {
        write!(
            fmtr,
            "{}",
            match *self {
                Gcc => "GCC",
                Clang => "Clang",
            }
        )
    }
}

impl Compiler {
    fn as_cmake_args(&self) -> [&'static str; 2] {
        match *self {
            Gcc => ["-DCMAKE_C_COMPILER=gcc", "-DCMAKE_CXX_COMPILER=g++"],
            Clang => ["-DCMAKE_C_COMPILER=clang", "-DCMAKE_CXX_COMPILER=clang++"],
        }
    }
}

enum BuildType {
    Debug,
    Release,
}

impl BuildType {
    fn as_cmake_arg(&self) -> &'static str {
        match *self {
            Debug => "-DCMAKE_BUILD_TYPE=Debug",
            Release => "-DCMAKE_BUILD_TYPE=Release",
        }
    }
}

use Compiler::*;
use BuildType::*;

struct Config {
    name: String,
    compiler: Compiler,
    build_type: BuildType,
    cmake_args: Vec<&'static str>,
}

fn config(name: &str, comp: Compiler, build_type: BuildType, args: &[&'static str]) -> Config {
    Config {
        name: format!("{}-{}", comp, name),
        compiler: comp,
        build_type: build_type,
        cmake_args: args.to_owned(),
    }
}

fn create_config(conf: &Config, build_system: BuildSystem, project_dir: &str) -> bool {
    use std::{fs, env};
    use std::process::Command;
    let parent_dir = env::current_dir().unwrap();
    fs::create_dir(&conf.name).unwrap();
    env::set_current_dir(&Path::new(&conf.name)).unwrap();
    let result = Command::new("cmake")
        .arg(project_dir)
        .arg(build_system.as_cmake_arg())
        .args(&conf.compiler.as_cmake_args())
        .arg(conf.build_type.as_cmake_arg())
        .args(&conf.cmake_args)
        .status()
        .unwrap();
    env::set_current_dir(&parent_dir).unwrap();
    result.success()
}

extern crate ansi_term;

fn print_usage(program: &str, opts: &Options) {
    let brief = format!("Usage: {} project_dir [options]", program);
    print!("{}", opts.usage(&brief));
}

struct CMakeListsProperties {
    has_sanitize: bool,
}

fn parse_cmakelists_txt(path: &Path) -> std::io::Result<CMakeListsProperties> {
    use std::fs::File;
    use std::io::Read;
    let mut f = File::open(path.join("CMakeLists.txt"))?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    let has_sanitize = match s.find("${SANITIZE}") {
        Some(_) => true,
        None => false,
    };
    Ok(CMakeListsProperties { has_sanitize: has_sanitize })
}

fn run() -> (i32, Option<String>) {
    let mut args = std::env::args();
    let mut opts = Options::new();
    let program = args.next().unwrap().clone();
    opts.optflag("", "no-sanitize", "Don't build sanitize configurations");
    opts.optflag(
        "",
        "no-ninja",
        "Don't use ninja as a build system. Use plain make instead.",
    );
    opts.optflag("h", "help", "print this help menu");
    let matches = match opts.parse(args) {
        Ok(m) => m,
        Err(e) => return (1, Some(format!("{}", e))),
    };
    if matches.opt_present("h") {
        print_usage(&program, &opts);
        return (1, None);
    }
    let arg = match matches.free.get(0) {
        Some(arg) => arg,
        None => {
            print_usage(&program, &opts);
            return (1, None);
        }
    };
    let proj_dir = std::env::current_dir().unwrap().join(&arg);
    match std::fs::metadata(&proj_dir) {
        Ok(_) => {}
        Err(e) => {
            return (
                1,
                Some(
                    format!(
                        "Error while trying to look up directory {:?}: {}",
                        proj_dir,
                        e
                    ),
                ),
            );
        }
    }
    let props = match parse_cmakelists_txt(&proj_dir) {
        Ok(props) => props,
        Err(e) => {
            return (
                1,
                Some(
                    format!("Failed to open CMakeLists.txt in {:?}: {}", proj_dir, e),
                ),
            );
        }
    };
    let build_dir = PathBuf::from(format!("build-{}", arg));
    if build_dir.exists() {
        return (
            1,
            Some(
                format!(
                    "The build directory ({:?}) already exists. Delete it first.",
                    build_dir
                ),
            ),
        );
    }
    std::fs::create_dir(&build_dir).unwrap();
    std::env::set_current_dir(&build_dir).unwrap();
    let mut configs = vec![
        config("Debug", Gcc, Debug, &[]),
        config("Release", Gcc, Release, &[]),
        config("Debug", Clang, Debug, &[]),
        config("Release", Clang, Release, &[]),
    ];
    if props.has_sanitize && !matches.opt_present("no-sanitize") {
        configs.extend(
            vec![
                config("Asan", Clang, Debug, &["-DSANITIZE=address"]),
                config("Ubsan", Clang, Debug, &["-DSANITIZE=undefined"]),
                config("Tsan", Clang, Debug, &["-DSANITIZE=thread"]),
            ],
        );
    }
    let build_system = if matches.opt_present("no-ninja") {
        BuildSystem::Make
    } else {
        BuildSystem::Ninja
    };
    for c in configs {
        use ansi_term::Colour::{Green, Yellow, White};
        println!(
            "{0} {1} {2} {0}",
            Green.bold().paint("==="),
            White.bold().paint("Creating configuration for"),
            Yellow.bold().paint(&c.name[..])
        );
        if !create_config(&c, build_system, proj_dir.to_str().unwrap()) {
            break;
        }
    }

    (0, None)
}

fn main() {
    let (retv, opt_msg) = run();
    if let Some(msg) = opt_msg {
        eprintln!("{}", msg);
    }
    std::process::exit(retv);
}
