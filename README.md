# depgraph
A command line tool that shows dependencies Rust modules have to other modules. 
Disclaimer: Mostly a proof of concept, does not work for many common cases (e.g. multiple modules within
a file)

Currently only supports top-level modules (e.g. `use some::module::path` becomes `some`). Works
by parsing `use` statements in the source files (via the `syn` library). 

Usage:
```
depgraph [FLAGS] [OPTIONS] <SRC_PATH>

FLAGS:
    -i, --ignore     Ignore external dependencies (extern crates)
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -o, --output <OUT_PATH>    Graphviz output file.

ARGS:
    <SRC_PATH>    Path to the src folder of the Rust project
```

If no output path is given, the tool will print the dependencies to the console. 

Example output: (for this project, which has only one file)
```
$ depgraph src/
Dependencies for module `main`:
        errors
        petgraph
        std
        walkdir
```