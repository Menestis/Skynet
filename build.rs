use std::{env, fs};
use std::collections::HashMap;
use std::fs::{ReadDir};
use codegen::{Function, Scope, Type};
use regex::Regex;

pub fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();


    // let bindings = bindgen::Builder::default()
    //     .header("libs/Murmur.hpp")
    //     .generate()
    //     .expect("Unable to generate bindings");
    //
    // let out_path = PathBuf::from("./src/mumble/bindings.rs");
    // bindings
    //     .write_to_file(out_path)
    //     .expect("Couldn't write bindings!");
    //


    let mut files = HashMap::new();

    explore_dir(fs::read_dir("./src").unwrap(), &mut files);


    let re = Regex::new("^ *//#\\[query\\(([a-z|_]+) ?= ?\"(.*)\"\\)\\] *$").unwrap();


    let mut queries = HashMap::new();

    for (_x, y) in files {
        for yy in y.lines() {
            for x in re.captures_iter(&yy) {
                // println!("cargo:warning={} <> {}", &x[1], &x[2]);
                queries.insert(x[1].to_string(), x[2].to_string());
            }
        }
    }

    let mut scope = Scope::new();
    scope.import("scylla::statement::prepared_statement", "PreparedStatement");
    scope.import("scylla", "Session");
    scope.import("scylla::transport::errors", "QueryError");
    scope.import("tracing", "instrument");
    scope.import("tracing", "trace");

    let st = scope.new_struct("Queries");
    st.vis("pub");

    for (x, _y) in &queries {
        st.field(&format!("pub {}", x), "PreparedStatement");
    }

    let im = scope.new_impl("Queries");

    let mut f = Function::new("new");

    f.vis("pub");
    f.set_async(true);
    f.attr("instrument(skip(s), level = \"debug\")");
    f.arg("s", Type::new("&Session"));
    f.ret(Type::new("Result<Self, QueryError>"));

    for (x, y) in &queries {
        f.line(format!("trace!(\"Preparing query {}\");", x));
        f.line(format!("let {} = s.prepare(\"{}\").await?;", x, y));
    }

    f.line("Ok(Queries {");
    for (x, _y) in &queries {
        f.line(format!("{},", x));
    }
    f.line("})");

    im.push_fn(f);

    fs::write(format!("{}/{}", out_dir, "queries.rs"), scope.to_string()).expect("Writing file !");

}

pub fn explore_dir(entry: ReadDir, map: &mut HashMap<String, String>) {
    for path in entry {
        let path = path.expect("Read path");
        let file_type = path.file_type().expect("File Type");
        if file_type.is_file() {
            map.insert(path.path().to_str().unwrap().to_string(), fs::read_to_string(path.path()).expect("Reading file"));
        } else if file_type.is_dir() {
            explore_dir(fs::read_dir(path.path()).expect("Reading directory"), map);
        }
    }
}