use argparse::Mode;

mod argparse;
mod rdp;
mod web;

fn main() {
    println!("Hello, world!");
    let opts = argparse::parse();

    println!("Got opts:\n{:?}", opts);

    match opts.mode {
        Mode::Rdp => rdp::capture(&opts),
        Mode::Web => web::capture(&opts),
    }
}
