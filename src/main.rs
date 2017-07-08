#[macro_use]
extern crate error_chain;
extern crate petgraph;
extern crate walkdir;
extern crate syn;

use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Read;

use walkdir::WalkDir;
use petgraph::prelude::*;

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

#[derive(Clone, Debug)]
pub struct UsePath {
    segments: Vec<String>,
}

impl UsePath {
    pub fn to_statement(&self) -> String {
        self.segments.join("::")
    }

    pub fn to_path<P>(&self, root: P) -> PathBuf where P: AsRef<Path> {
        let mut path = PathBuf::new();
        let len = self.segments.len();
        if len == 1 {
            PathBuf::from(format!("{}.rs", self.segments[0]))
        } else {
            for segment in self.segments.iter().take(len - 1) {
                path.push(segment);
            }
            path.push(format!("{}.rs", self.segments.last().unwrap()));
            root.as_ref().join(&path)
        }
    }

    pub fn is_extern<P>(&self, root: &P) -> bool where P: AsRef<Path> {
        self.to_path(root).exists()
    }
}

impl From<syn::ItemUse> for UsePath {
    fn from(item_use: syn::ItemUse) -> UsePath {
        let mut segments = vec![];
        let path = match *item_use.path {
            syn::ViewPath::Simple(ref p) => &p.path,
            syn::ViewPath::Glob(ref p) => &p.path,
            syn::ViewPath::List(ref p) => &p.path,
        };
        for segment in &path.segments {
            segments.push(segment.item().ident.to_string())
        }

        UsePath { segments }
    }
}

pub fn file_to_ast<P>(path: P) -> Result<syn::File>
where
    P: AsRef<Path>,
{
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let ast = syn::parse_file(&content)?;
    Ok(ast)
}

fn extract_use_statements(file: &syn::File, depth: Option<usize>) -> Vec<UsePath> {
    let mut statements = vec![];
    for item in &file.items {
        match item.node {
            syn::ItemKind::Use(ref item_use) => {
                let path = UsePath::from(item_use.clone());
                statements.push(path);
            }
            _ => (),
        }
    }

    if let Some(n) = depth { 
        statements[..n].to_vec()
    } else {
        statements
    }
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
    let mut graph = Graph::new();
    for entry in WalkDir::new(&root_path) {
        let entry = entry?;
        if !is_rust_file(&entry) {
            continue;
        }
        let path = entry.path();
        let file = file_to_ast(path)?;
        let uses = extract_use_statements(&file, module_depth);
        let module_path = path.strip_prefix(&root_path)?;
        for statement in uses {
            println!("{}: {}", module_path.display(), statement.to_statement());
        }
    }
    Ok(graph)
}

fn main() {
    let graph = build_dependency_graph("../link-collector/src", Some(1)).unwrap();
    println!("{:?}", graph);
}
