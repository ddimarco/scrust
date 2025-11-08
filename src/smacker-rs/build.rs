fn main() {
    cc::Build::new()
        .file("src/c/libsmacker-code/smk_bitstream.c")
        .file("src/c/libsmacker-code/smk_hufftree.c")
        .file("src/c/libsmacker-code/smacker.c")
        .compile("smk");
}
