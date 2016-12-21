extern crate gcc;

fn main() {
    // gcc::Config::new()
    //     .file("src/c/libsmacker-code/smk_bitstream.c")
    //     .file("src/c/libsmacker-code/smk_hufftree.c")
    //     .file("src/c/libsmacker-code/smacker.c")
    //     .include("src/c/libsmacker-code")
    //     .pic(true)
    //     .compile("libsmk.a");
    gcc::compile_library("libsmk.a", &["src/c/libsmacker-code/smk_bitstream.c",
                                       "src/c/libsmacker-code/smk_hufftree.c",
                                       "src/c/libsmacker-code/smacker.c"]);
    // println!("cargo:rustc-link-lib=static=smk");
}
