use anyhow::{anyhow, Context, Result};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use kube::config::Kubeconfig;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err(anyhow!("Expected exactly one argument, got {}", args.len()));
    }
    let new_context = &args[1];

    let location = match env::var("KUBECONFIG") {
        Ok(value) if !value.is_empty() => PathBuf::from(value),
        _ => match env::var("HOME") {
            Ok(value) if !value.is_empty() => PathBuf::from(value).join(".kube").join("config"),
            _ => return Err(anyhow!("HOME environment variable empty or unset")),
        },
    };

    let mut kubeconfig_raw = String::new();
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&location)
        .context(format!("Reading {:?}", location))?;
    file.read_to_string(&mut kubeconfig_raw)?;

    let kubeconfig = serde_yaml::from_str::<Kubeconfig>(&kubeconfig_raw)?;

    if args[0] == "cn" || args[0].ends_with("/cn") {
        update_namespace(kubeconfig, &mut file, new_context)
    } else {
        update_context(kubeconfig, &mut file, new_context)
    }
}

fn update_namespace(mut kubeconfig: Kubeconfig, file: &mut File, namespace: &String) -> Result<()> {
    if kubeconfig.current_context.is_none() {
        println!("No current context set, can not to update namespace");
        return Err(anyhow!(
            "No current context set, can not to update namespace"
        ));
    }

    for context in kubeconfig.contexts.iter_mut() {
        if context.name == *kubeconfig.current_context.as_ref().unwrap() {
            if context
                .context
                .as_ref()
                .map_or(false, |ctx| ctx.namespace == Some(namespace.to_string()))
            {
                println!("Already in namespace {}", namespace);
                return Ok(());
            }
            context.context.as_mut().unwrap().namespace = Some(namespace.to_string());
            break;
        }
    }

    update_kubeconfig(kubeconfig, file)?;

    println!("updated namespace to {}", namespace);

    Ok(())
}

fn update_context(mut kubeconfig: Kubeconfig, file: &mut File, new_context: &str) -> Result<()> {
    if kubeconfig
        .current_context
        .map(|s| s == new_context)
        .unwrap_or(false)
    {
        println!("Already in context {}", new_context);
        return Ok(());
    }

    let mut found = false;
    for context in &kubeconfig.contexts {
        if context.name == *new_context {
            found = true;
            break;
        }
    }
    if !found {
        return Err(anyhow!(
            "Context {} does not exist, refusing to update kubeconfig",
            new_context
        ));
    }

    kubeconfig.current_context = Some(new_context.to_string());

    update_kubeconfig(kubeconfig, file)?;

    println!("Switched to context {}", new_context);

    Ok(())
}

fn update_kubeconfig(kubeconfig: Kubeconfig, file: &mut File) -> Result<()> {
    let updated_kubeconfig = serde_yaml::to_string(&kubeconfig)?;

    file.seek(SeekFrom::Start(0)).unwrap();
    file.set_len(0).unwrap();
    file.write_all(updated_kubeconfig.as_bytes()).unwrap();

    Ok(())
}
