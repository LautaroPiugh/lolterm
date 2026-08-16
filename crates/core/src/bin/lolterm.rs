fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match lolterm_core::cli::run(&args) {
        Ok(code) => std::process::exit(code),
        Err(err) => {
            eprintln!("lolterm: {err}");
            std::process::exit(2);
        }
    }
}
