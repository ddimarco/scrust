extern crate gcc;

fn main() {
    gcc::compile_library("libsmk.a", &["src/c/libsmacker-code/smk_bitstream.c",
                                       "src/c/libsmacker-code/smk_hufftree.c",
                                       "src/c/libsmacker-code/smacker.c"]);
}
