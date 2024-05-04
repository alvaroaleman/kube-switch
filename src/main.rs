use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process;

use kube::config::Kubeconfig;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Expected exactly one argument, got {}", args.len());
        process::exit(1)
    }
    let new_context = &args[1];

    let mut location = String::new();

    if let Ok(value) = env::var("KUBECONFIG") {
        if !value.is_empty() {
            location = value;
        }
    } else {
        let home = env::var("HOME").unwrap();
        location = PathBuf::from(home)
            .join(".kube")
            .join("config")
            .to_str()
            .unwrap()
            .to_string();
    }

    println!("Kubeconfig location: {}", location);

    let mut kubeconfig_raw = String::new();
    File::open(&location)
        .unwrap()
        .read_to_string(&mut kubeconfig_raw)
        .unwrap();

    let mut kubeconfig = serde_yaml::from_str::<Kubeconfig>(&kubeconfig_raw).unwrap();

    if kubeconfig
        .current_context
        .map(|s| s == *new_context)
        .unwrap_or(false)
    {
        println!("Already in context {}", new_context);
        process::exit(0);
    }

    kubeconfig.current_context = Some(new_context.to_string());

    let updated_kubeconfig = serde_yaml::to_string(&kubeconfig).unwrap();

    File::create(location)
        .unwrap()
        .write(updated_kubeconfig.as_bytes())
        .unwrap();
}
