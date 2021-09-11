mod models;
mod hosts;
mod minikube;
mod k8s;
mod docker;

use docker::build_service_image;
use k8s::ensure_tag_config;
use minikube::get_minikube_ip;
use k8s::build_routing_definition;
use k8s::build_service_cluster_definition;
use k8s::kubectl_delete_all;
use k8s::kubectl_apply;
use minikube::get_host_ip;
use k8s::build_service_local_definition;
use k8s::K8sServiceConfig;
use std::io;
use std::process::Command;
use std::collections::HashMap;
use std::path::Path;
use std::env;
use std::fs;
use std::str;
use std::io::ErrorKind;
extern crate yaml_rust;

use models::*;
use hosts::{check_etc_hosts, patch_etc_hosts};

fn locate_project() -> Option<String> {
    let mut path = env::current_dir().ok()?;
    loop {
        if path.join("minitel.yaml").is_file() {
            return Some(String::from(path.to_str()?));
        };
        if !path.pop() {
            return None;
        }
    }
}

fn load_definition(path: &str) -> Definition {
    let contents = fs::read_to_string(Path::new(path).join("minitel.yaml"))
        .expect("Could not read minitel file");
    let parsed: Definition = serde_yaml::from_str(&contents).unwrap();

    return parsed;
}

fn load_config(path: &str) -> HashMap<String, ServiceConfig> {
    let parsed = fs::read_to_string(Path::new(path).join("minitel.local.yaml"))
        .map(|contents| serde_yaml::from_str(&contents).unwrap());

    match parsed {
        Ok(result) => result,
        _ => {
            println!("No local config found. Will use defaults.");
            HashMap::new()
        }
    }
}

fn deploy_to_cluster(global_config: &GlobalConfig, service: &Service) {

    build_service_image(&global_config.project_location, &global_config.app_name, &service.name);

    let config = build_service_cluster_definition(& K8sServiceConfig {
        host_ip: &global_config.host_ip,
        service_name: &service.name,
        service_port: service.port,
        app_name: global_config.app_name,
        config_hash: &global_config.config_hash
    });

    kubectl_apply(&config);
}

fn deploy_local(global_config: &GlobalConfig, service: &Service) {
    
    let config = build_service_local_definition(& K8sServiceConfig {
        host_ip: &global_config.host_ip,
        service_name: &service.name,
        service_port: service.port,
        app_name: global_config.app_name,
        config_hash: &global_config.config_hash
    });

    kubectl_apply(&config);
}

fn start_service(global_config: &GlobalConfig, service: &Service, config: &ServiceConfig) {
    match config.deploy {
        DeployMode::Cluster => deploy_to_cluster(global_config, service),
        DeployMode::Local => deploy_local(global_config, service)
    }
}

fn run_patch_hosts_as_sudo() {
    let program = env::args().nth(0).expect("Bad cli arguments");

    println!("I need to patch /etc/hosts to map the app's hostname to the minikube ip address.");
    println!("This requires root access. You might be prompted for your password...");

    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("sudo {} patch-hosts", program))
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        println!("Failed to patch hosts file");
    }
}

fn run_down() {
    println!("Destroying all state...");

    // TODO: Clear etc hosts

    kubectl_delete_all();

    println!("Done.");
}

fn configure_routing(def: &Definition, cfg: &HashMap<String, ServiceConfig>) -> String {
    let hash = format!("{}", rand::random::<u16>());
    let config = build_routing_definition(def, cfg, &hash);
    kubectl_apply(&config);

    hash
}

fn validate_and_apply_extras(project_location: &str) {
    let kubeconfigs = fs::read_dir(Path::new(project_location).join(".minitel/kube"));

    if let Ok(entries) = kubeconfigs {
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_file() {
                let config = fs::read_to_string(path).unwrap();
                kubectl_apply(&ensure_tag_config(config));
            }
        }
    }

}

fn run_up() {

    let project_location = locate_project().expect("Could not find project root.");
    let def = load_definition(&project_location);
    let cfg = load_config(&project_location);


    let config_hash = configure_routing(&def, &cfg);

    let config = GlobalConfig {
        host_ip: &get_host_ip(),
        minikube_ip: &get_minikube_ip(),
        app_name: &def.app_name,
        config_hash: &config_hash,
        project_location: &project_location
    };

    let default_cfg = ServiceConfig {
        deploy: DeployMode::Cluster
    };

    if let Ok(false) = check_etc_hosts(&config, &def) {
        run_patch_hosts_as_sudo();
    }


    validate_and_apply_extras(&project_location);

    for service in def.services {
        start_service(&config, &service, cfg.get(&service.name).get_or_insert(&default_cfg));
    }
}

fn run_patch_hosts() {
    let project_location = locate_project().expect("Could not find project root.");
    let def = load_definition(&project_location);

    let config = GlobalConfig {
        host_ip: &get_host_ip(),
        minikube_ip: &get_minikube_ip(),
        app_name: &def.app_name,
        config_hash: "dont care",
        project_location: &project_location
    };

    patch_etc_hosts(&config, &def).expect("Failed to update hosts file!");
}

fn run_help() {
    println!("usage: minitel <up | down>")
}

fn main() -> Result<(), io::Error> {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1);

    if cmd.map_or_else(|| false, |v| v.eq("up")) {
        run_up();
    } else if cmd.map_or_else(|| false, |v| v.eq("down")) {
        run_down();
    } else if cmd.map_or_else(|| false, |v| v.eq("patch-hosts")) {
        run_patch_hosts();
    } else {
        run_help();
        return Err(io::Error::new(ErrorKind::InvalidInput, "Invalid arguments"))
    }

    Ok(())
}
