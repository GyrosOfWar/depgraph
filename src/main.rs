#[macro_use]
extern crate clap;
extern crate failure;
extern crate petgraph;
extern crate syn;
extern crate walkdir;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use petgraph::dot::{Config, Dot};
use petgraph::prelude::*;
use walkdir::WalkDir;

type Result<T> = std::result::Result<T, failure::Error>;

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
    use syn::visit::Visit;

    struct Visitor {
        statements: HashSet<String>,
    }

    impl<'ast> Visit<'ast> for Visitor {
        fn visit_use_tree(&mut self, tree: &syn::UseTree) {
            if let syn::UseTree::Path(tree) = tree {
                self.statements.insert(tree.ident.to_string());
            }
        }
    }
    let mut visitor = Visitor {
        statements: Default::default(),
    };
    syn::visit::visit_file(&mut visitor, file);

    visitor.statements
}

fn is_external_dependency<P>(root: P, module: &str) -> bool
where
    P: AsRef<Path>,
{
    let root = root.as_ref();
    let file = root.join(&format!("{}.rs", module));
    let module = root.join(module);
    !(file.is_file() || module.is_dir())
}

fn is_rust_file(e: &walkdir::DirEntry) -> bool {
    e.path().extension().map(|e| e == "rs").unwrap_or(false)
}

fn module_from_path(root_path: &Path, path: &Path) -> Result<String> {
    let relative_path = path.strip_prefix(root_path)?;
    let module_name = Path::new(relative_path.iter().nth(0).unwrap());
    Ok(module_name
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .into_owned())
}

fn build_dependency_graph<P>(root_path: P, ignore_extern: bool) -> Result<Graph<String, ()>>
where
    P: AsRef<Path>,
{
    use std::ffi::OsStr;

    let mut graph = Graph::new();
    let mut nodes = HashMap::new();

    let walker = WalkDir::new(&root_path)
        .into_iter()
        .filter_entry(|e| e.path().file_name() != Some(OsStr::new("target")));
    for entry in walker {
        let entry = entry?;
        if !is_rust_file(&entry) {
            continue;
        }
        let path = entry.path();
        let file = file_to_ast(path)?;
        let modules = extract_used_modules(&file);
        let this_module = module_from_path(root_path.as_ref(), path)?;

        let from_idx = *nodes
            .entry(this_module.clone())
            .or_insert_with(|| graph.add_node(this_module.clone()));
        for module in &modules {
            if ignore_extern && is_external_dependency(&root_path, module) {
                continue;
            }
            let to_idx = *nodes
                .entry(module.clone())
                .or_insert_with(|| graph.add_node(module.clone()));
            if graph.find_edge(from_idx, to_idx).is_none() {
                graph.add_edge(from_idx, to_idx, ());
            }
        }
    }
    Ok(graph)
}

fn run() -> Result<()> {
    let matches = clap_app!(depgraph =>
        (version: "0.1")
        (author: "Martin Tomasi <martin.tomasi@gmail.com>")
        (about: "Shows a dependency graph for Rust projects")
        (@arg IGNORE_EXTERNAL: -i --ignore "Ignore external dependencies (extern crates)")
        (@arg OUT_PATH: +takes_value -o --output "Graphviz output file.")
        (@arg SRC_PATH: +required "Path to the src folder of the Rust project")
    )
    .get_matches();

    let path = matches.value_of("SRC_PATH").unwrap();
    let ignore_external = matches.is_present("IGNORE_EXTERNAL");
    let graph = build_dependency_graph(path, ignore_external)?;
    if !matches.is_present("OUT_PATH") {
        for idx in graph.node_indices() {
            let node = &graph[idx];
            let mut neighbors: Vec<_> = graph.neighbors(idx).map(|n| &graph[n]).collect();
            neighbors.sort();
            if !neighbors.is_empty() {
                println!("Dependencies for module `{}`:", node);
                for neighbor in neighbors {
                    println!("\t{}", neighbor);
                }
            }
        }
    } else {
        let path = matches.value_of("OUT_PATH").unwrap();
        let mut file = File::create(path)?;
        write!(
            file,
            "{:?}",
            Dot::with_config(&graph, &[Config::EdgeNoLabel])
        )?;
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        println!("Error: {}", e);
    }
}
