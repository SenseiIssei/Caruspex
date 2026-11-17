//! caruspex-j2534-bridge: built for 32-bit Windows (i686-pc-windows-msvc),
//! it loads an old 32-bit PassThru driver for the 64-bit Caruspex.
//! Caruspex starts it as `caruspex-j2534-bridge --pipe \\.\pipe\<name>` and
//! talks to it over that pipe; it ends when Caruspex closes the pipe.

// No console window when Caruspex starts it.
#![windows_subsystem = "windows"]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.as_slice() {
        [flag, name] if flag == "--pipe" => run(name),
        _ => 2,
    };
    std::process::exit(code);
}

#[cfg(windows)]
fn run(name: &str) -> i32 {
    match caruspex_j2534::serve_pipe(name) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

#[cfg(not(windows))]
fn run(_name: &str) -> i32 {
    2 // PassThru drivers exist on Windows only
}
