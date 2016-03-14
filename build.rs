// build.rs

// Bring in a dependency on an externally maintained `gcc` package which manages
// invoking the C compiler.
extern crate gcc;

// use std::env::set_var;

fn main() {
    gcc::Config::new()
                .file("cpp/basic.cpp")
                .cpp(true)
                .compile("libminimal.a");    
}

//    let conf = gcc::Config::new().cpp(true);
//    gcc::compile_library("libminimal.a", &["minimal.cpp"]);

//    gcc::compile_library("libminimal.a", &["cpp/minimal.cpp"]);
//    println!("cargo:rustc-flags=-l dylib=stdc++");

/*
    set_var("LIBFLAGS", "-fPIC");
    set_var("LDFLAGS", "-pthread");
    set_var("CXXFLAGS", "-O2");
    gcc::Config::new()
                .file("minimal.cpp")
                .cpp(true)
//                .include("src")
                .compile("libminimal.a");    
*/

/*    let conf = gcc::new();
    conf.cpp(true);
    gcc::compile_library("libminimal.a", &["minimal.cpp"]);
*/


