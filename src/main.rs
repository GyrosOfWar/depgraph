#[macro_use]
extern crate clap;
#[macro_use]
extern crate error_chain;
extern crate petgraph;
extern crate syn;
extern crate walkdir;

use std::collections::{HashSet, HashMap};
use std::ffi::OsStr;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use petgraph::dot::{Dot, Config};
use petgraph::prelude::*;
use walkdir::WalkDir;

use errors::Result;

mod errors {
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
            Syn(::syn::ParseError);
            WalkDir(::walkdir::Error);
            Prefix(::std::path::StripPrefixError);
        }
    }
}

fn file_to_ast<P>(path: P) -> Result<syn::File>
where
    P: AsRef<Path>,
{
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let ast = syn::parse_file(&content)?;
    Ok(ast)
}

fn extract_used_modules(file: &syn::File) -> HashSet<String> {
    let mut statements = HashSet::new();
    for item in &file.items {
        match item.node {
            syn::ItemKind::Use(ref item_use) => {
                let path = match *item_use.path {
                    syn::ViewPath::Simple(ref p) => &p.path,
                    syn::ViewPath::Glob(ref p) => &p.path,
                    syn::ViewPath::List(ref p) => &p.path,
                };
                match path.segments.iter().nth(0) {
                    Some(s) => statements.insert(s.item().ident.to_string()),
                    None => continue,
                };
            }
            _ => (),
        }
    }
    statements
}

fn is_external_dependency<P>(root: P, module: &str) -> bool
where
    P: AsRef<Path>,
{
    let root = root.as_ref();
    let file = root.join(&format!("{}.rs", module));
    let module = root.join(module);
    if file.is_file() {
        false
    } else if module.is_dir() {
        false
    } else {
        true
    }
}

fn is_rust_file(e: &walkdir::DirEntry) -> bool {
    e.path().extension().map(|e| e == "rs").unwrap_or(false)
}

fn build_dependency_graph<P>(root_path: P, ignore_extern: bool) -> Result<Graph<String, ()>>
where
    P: AsRef<Path>,
{
    let mut graph = Graph::new();
    let mut nodes: HashMap<String, NodeIndex> = HashMap::new();

    for entry in WalkDir::new(&root_path) {
        let entry = entry?;
        if !is_rust_file(&entry) {
            continue;
        }
        let path = entry.path();
        let file = file_to_ast(path)?;
        let modules = extract_used_modules(&file);
        let this_module = path.strip_prefix(&root_path)?;
        let this_module = Path::new(this_module.iter().nth(0).unwrap());

        let this_module = if this_module.extension() == Some(OsStr::new("rs")) {
            let s = format!("{}", this_module.display());
            let l = s.len();
            s[..l - 3].to_string()
        } else {
            format!("{}", this_module.display())
        };

        let from_idx = nodes
            .entry(this_module.clone())
            .or_insert_with(|| graph.add_node(this_module.clone()))
            .clone();
        for module in &modules {
            if ignore_extern && is_external_dependency(&root_path, &module) {
                continue;
            }
            let to_idx = nodes
                .entry(module.clone())
                .or_insert_with(|| graph.add_node(module.clone()))
                .clone();
            if graph.find_edge(from_idx, to_idx).is_none() {
                graph.add_edge(from_idx, to_idx, ());
            }
        }
    }
    Ok(graph)
}

fn run() -> Result<()> {
    let matches =  clap_app!(depgraph => 
        (version: "0.1")
        (author: "Martin Tomasi <martin.tomasi@gmail.com>")
        (about: "Shows a dependency graph for Rust projects")
        (@arg IGNORE_EXTERNAL: -i --ignore-external "Ignore external dependencies (extern crates)")
        (@arg OUT_PATH: +takes_value -o --output "Output file.")
        (@arg SRC_PATH: +required "Path to the src folder of the Rust project")
    ).get_matches();

    let path = matches.value_of("SRC_PATH").unwrap();
    let ignore_external = matches.is_present("IGNORE_EXTERNAL");
    let graph = build_dependency_graph(path, ignore_external)?;
    if !matches.is_present("OUT_PATH") {
        println!("{:?}", Dot::with_config(&graph, &[Config::EdgeNoLabel]))
    } else {
        let path = matches.value_of("OUT_PATH").unwrap();
        let mut file = File::create(path)?;
        write!(file, "{:?}", Dot::with_config(&graph, &[Config::EdgeNoLabel]))?;
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        println!("Error: {}", e);
    }
}
