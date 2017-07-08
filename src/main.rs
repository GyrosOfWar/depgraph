#[macro_use]
extern crate error_chain;
extern crate petgraph;
extern crate walkdir;
extern crate syn;

use std::path::Path;
use std::fs::File;
use std::io::Read;

use errors::Result;

mod errors {
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
            Syn(::syn::ParseError);
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

fn main() {
    let code = file_to_ast("src/main.rs").unwrap();
    for item in &code.items {
        match item.node {
            syn::ItemKind::Use(ref path) => {
                println!("Found use stmt");
            }
            _ => (),
        }
    }
}
