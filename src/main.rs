#![feature(path, path_ext, collections)]

extern crate getopts;

use getopts::Options;
use std::path::Path;

enum Compiler {
    Gcc,
    Clang
}

use std::fmt::{Display, Formatter};

impl Display for Compiler {
    fn fmt(&self, fmtr: &mut Formatter) -> Result<(), std::fmt::Error> {
        write!(fmtr, "{}", 
        match *self {
            Gcc => "GCC",
            Clang => "Clang"
        }
        )
    }
}

impl Compiler {
    fn as_cmake_args(&self) -> [&'static str; 2] {
        match *self {
            Gcc => ["-DCMAKE_C_COMPILER=gcc", "-DCMAKE_CXX_COMPILER=g++"],
            Clang => ["-DCMAKE_C_COMPILER=clang", "-DCMAKE_CXX_COMPILER=clang++"]
        }
    }
}

enum BuildType {
    Debug,
    Release
}

impl BuildType {
    fn as_cmake_arg(&self) -> &'static str {
        match *self {
            Debug => "-DCMAKE_BUILD_TYPE=Debug",
            Release => "-DCMAKE_BUILD_TYPE=Release"
        }
    }
}

use Compiler::*;
use BuildType::*;

struct Config {
    name: String,
    compiler: Compiler,
    build_type: BuildType,
    cmake_args: Vec<String>
}

fn config(name: &str, comp: Compiler, build_type: BuildType,
          args: &[&'static str]) -> Config {
    use std::borrow::ToOwned;
    let name = format!("{}-{}", comp, name);
    let args: Vec<String> = args.iter()
                                .map(|&x| -> String x.to_owned())
                                .collect();
    Config {
        name: name,
        compiler: comp,
        build_type: build_type,
        cmake_args: args
    }
}

fn create_config(conf: &Config, project_dir: &str) {
    use std::{fs, env};
    use std::process::Command;
    let parent_dir = env::current_dir().unwrap();
    fs::create_dir(&conf.name).unwrap();
    env::set_current_dir(&Path::new(&conf.name)).unwrap();
    Command::new("cmake").arg(project_dir)
                         .arg("-GCodeBlocks - Ninja")
                         .args(&conf.compiler.as_cmake_args())
                         .arg(conf.build_type.as_cmake_arg())
                         .args(&conf.cmake_args)
                         .status()
                         .unwrap();
    env::set_current_dir(&parent_dir).unwrap();
}

extern crate ansi_term;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    use std::fs::PathExt;
    let mut args = std::env::args();
    let mut opts = Options::new();
    let program = args.next().unwrap().clone();
    opts.optflag("", "no-sanitize",
                     "Don't build sanitize configurations");
    opts.optflag("h", "help", "print this help menu");
    let matches = match opts.parse(args) {
        Ok(m) => m,
        Err(e) => panic!("{}", e)
    };
    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }
    let arg = matches.free.get(0).expect("Needs project dir as argument!");
    let abs = std::env::current_dir().unwrap().join(&arg);
    let proj_dir = abs;
    if !proj_dir.exists() {
        panic!("Directory {:?} does not exist.", proj_dir);
    }
    let build_dir_string = "build-".to_string() + &arg;
    let build_dir = std::path::Path::new(&build_dir_string);
    std::fs::create_dir(&build_dir).unwrap();
    std::env::set_current_dir(&Path::new(build_dir.to_str().unwrap())).unwrap();
    let mut configs = vec![
        config("Debug", Gcc, Debug, &[]),
        config("Release", Gcc, Release, &[]),
        config("Debug", Clang, Debug, &[]),
        config("Release", Clang, Release, &[])
    ];
    if !matches.opt_present("no-sanitize") {
        configs.append(&mut vec![
        config("Asan", Clang, Debug, &["-DSANITIZE=address"]),
        config("Ubsan", Clang, Debug, &["-DSANITIZE=undefined"]),
        config("Tsan", Clang, Debug, &["-DSANITIZE=thread"])]);
    }
    for c in configs {
        use ansi_term::Colour::{Green, Yellow, White};
        println!("{0} {1} {2} {0}",
            Green.bold().paint("==="),
            White.bold().paint("Creating configuration for"),
            Yellow.bold().paint(&c.name));
        create_config(&c, proj_dir.to_str().unwrap());
    }
    
}
