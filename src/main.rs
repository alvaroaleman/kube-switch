use anyhow::{anyhow, Context, Result};
use k8s_openapi::api::core::v1::Namespace;
use kube::{
    api::{Api, ListParams},
    Client, ResourceExt,
};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use kube::config::Kubeconfig;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Opts {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    ChangeNamespace {
        namespace: String,
    },
    ChangeContext {
        context: String,
    },
    Complete {
        command: String,
        prefix: String,
        last_full_word: String,
    },
    /// Prints completion commands, add to your shell by executing `source <(kube-switch completion)`
    Completion {},
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::parse();
    match &opts.command {
        Commands::ChangeNamespace { namespace } => {
            let (kubeconfig, mut file) = get_kubeconfig()?;
            update_namespace(kubeconfig, &mut file, namespace)?;
            Ok(())
        }
        Commands::ChangeContext { context } => {
            let (kubeconfig, mut file) = get_kubeconfig()?;
            update_context(kubeconfig, &mut file, context)?;
            Ok(())
        }
        Commands::Complete {
            prefix,
            last_full_word,
            ..
        } => {
            if last_full_word == "cn" {
                for namespace in get_namespaces().await? {
                    if !namespace.starts_with(prefix) {
                        continue;
                    }
                    println!("{}", namespace);
                }
                return Ok(());
            }
            if last_full_word == "sc" {
                let (kubeconfig, _) = get_kubeconfig()?;
                for context in kubeconfig.contexts {
                    if !context.name.starts_with(prefix) {
                        continue;
                    }
                    println!("{}", context.name);
                }
            };
            Ok(())
        }
        Commands::Completion {} => {
            println!(
                r#"alias cn="kube-switch change-namespace"
alias sc="kube-switch change-context"
complete -C "kube-switch complete" sc
complete -C "kube-switch complete" cn
"#
            );
            Ok(())
        }
    }
}

async fn get_namespaces() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let client = Client::try_default().await?;

    let mut namespace_names: Vec<String> = Vec::new();
    let namespaces: Api<Namespace> = Api::all(client);
    for ns in namespaces.list(&ListParams::default()).await? {
        namespace_names.push(ns.name_any());
    }

    Ok(namespace_names)
}

fn get_kubeconfig() -> Result<(Kubeconfig, File)> {
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

    let kubeconfig = match kubeconfig_raw.chars().next() {
        Some('{') => serde_json::from_str::<Kubeconfig>(&kubeconfig_raw)?,
        _ => serde_yaml::from_str::<Kubeconfig>(&kubeconfig_raw)?,
    };

    Ok((kubeconfig, file))
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

    println!("Updated namespace to {}", namespace);

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
    let updated_kubeconfig = serde_json::to_string(&kubeconfig)?;

    file.seek(SeekFrom::Start(0)).unwrap();
    file.set_len(0).unwrap();
    file.write_all(updated_kubeconfig.as_bytes()).unwrap();

    Ok(())
}
