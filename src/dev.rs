
use crate::Service;
use crate::ServiceConfig;
use crate::DeployMode;
use std::collections::HashMap;
use crate::Definition;
use std::process::Command;
use std::path::PathBuf;
use std::path::Path;

pub fn build_development_env(def: &Definition, cfg: &HashMap<String, ServiceConfig>) -> HashMap<String, String> {

    def.services.iter().map(|service| {
        let name = format!("{}_SERVICE", service.name.to_uppercase());
        match cfg.get(&service.name).map(|s| &s.deploy).get_or_insert(&DeployMode::Cluster) {
            DeployMode::Cluster => {
                let api_root = match &service.cluster_prefix {
                    Some(e) => String::from(e),
                    None => String::from("/")
                };
                (name, format!("http://{}{}", def.hostname, &api_root))
            }
            _ => {
                (name, format!("http://localhost:{}", service.port))
            }
        }
    }).collect()
}

pub fn start_development_service(project_location: &str, service: &Service, env: &HashMap<String, String>) {
    let script = get_script(project_location, &service.name, "start.sh");
    run_script(&script, service.port, env);
}

pub fn stop_development_service(project_location: &str, service: &Service, env: &HashMap<String, String>) {
    let script = get_script(project_location, &service.name, "stop.sh");
    run_script(&script, service.port, env);
}

fn get_script(project_location: &str, service_name: &str, script: &str) -> PathBuf {
    Path::new(project_location).join("services").join(service_name).join(script)
}

fn run_script(script: &Path, port: i32, env: &HashMap<String, String>) {
    let status = Command::new(script)
        .current_dir(script.parent().unwrap())
        .envs(env)
        .env("PORT", format!("{}", port))
        .spawn()
        .expect("Failed to execute command")
        .wait()
        .expect("Command failed");

    if !status.success() {
        println!("[WARNING] Failed to run script {}", script.to_str().unwrap());
    }
}