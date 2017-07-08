#[macro_use]
extern crate error_chain;
extern crate petgraph;
extern crate walkdir;
extern crate syn;

use std::path::Path;
use std::fs::File;
use std::io::Read;

use walkdir::{WalkDir, WalkDirIterator};
use petgraph::prelude::*;

use errors::Result;

mod errors {
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
            Syn(::syn::ParseError);
            WalkDir(::walkdir::Error);
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

fn path_from_use_item(item_use: &syn::ItemUse) -> Vec<&syn::Ident> {
    let mut paths = vec![];
    let path = match *item_use.path {
        syn::ViewPath::Simple(ref p) => &p.path,
        syn::ViewPath::Glob(ref p) => &p.path,
        syn::ViewPath::List(ref p) => &p.path,
    };
    for segment in &path.segments {
        paths.push(&segment.item().ident)
    }

    paths
}

fn use_statements(file: &syn::File) -> Vec<String> {
    let mut statements = vec![];
    for item in &file.items {
        match item.node {
            syn::ItemKind::Use(ref item_use) => {
                let paths = path_from_use_item(item_use);
                let stmt = paths
                    .into_iter()
                    .map(|i| i.as_ref())
                    .collect::<Vec<_>>()
                    .join("::");
                statements.push(stmt);
            }
            _ => (),
        }
    }

    statements
}

fn is_rust_file(e: &walkdir::DirEntry) -> bool {
    e.path().extension().map(|e| e == "rs").unwrap_or(false)
}

fn build_dependency_graph<P>(
    root_path: P,
    module_depth: Option<usize>,
) -> Result<Graph<String, String>>
where
    P: AsRef<Path>,
{
    let iter = WalkDir::new(root_path).into_iter();
    let mut graph = Graph::new();

    for entry in iter {
        let entry = entry?;
        if !is_rust_file(&entry) {
            continue;
        }
        let path = entry.path();
        println!("{}", path.display());
        let file = file_to_ast(path)?;
        let statements = use_statements(&file);
        graph.add_node(format!("{}", path.display()));
        for use_stmt in statements {
            println!("{}", use_stmt);
        }
    }
    Ok(graph)
}

fn main() {
    let graph = build_dependency_graph("src", None).unwrap();
    println!("{:?}", graph);
}
