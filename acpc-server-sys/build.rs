use std::env;
use std::path::PathBuf;

fn main() {
    // Cのコードをコンパイルします。
    //    ccクレートはcargoにコンパイルのリンクオプションを自動的に追加します。
    //    これの意味することは、本来Cのコードをビルドした後にRustのコードにリンクするために必要な
    //    以下の記述をbuild.rsから省略できるということです。
    //     println!("cargo:rustc-link-search=native={}", env::var("OUT_DIR").unwrap());
    //     println!("cargo:rustc-link-lib=fibonacci");
    cc::Build::new()
        .warnings(true)
        .flag("-Wall")
        .flag("-Wextra")
        .file("src/c/game.c")
        .include("src/c")
        .compile("libgame.a");

    cc::Build::new()
        .warnings(true)
        .flag("-Wall")
        .flag("-Wextra")
        .file("src/c/rng.c")
        .include("src/c")
        .compile("librng.a");

    // bindgenにgame.hの場所を伝えます。
    let bindings = bindgen::Builder::default()
        .header("src/c/game.h")
	.header("src/c/rng.h")
	.header("src/c/eval_hands_table.h")
        .generate()
        .expect("Unable to generate bindings!");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    // bindgenによってbindings.rsという名前でRustのバインディングコードが生成されます。
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");    
}
