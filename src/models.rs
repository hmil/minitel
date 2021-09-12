
use serde::{Deserialize};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct Definition {
    pub services: Vec<Service>,
    pub hostname: String,
    pub app_name: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct Service {
    pub name: String,
    pub port: i32,
    pub cluster_prefix: Option<String>
}

pub struct GlobalConfig<'a> {
    pub host_ip: &'a str,
    pub minikube_ip: &'a str,
    pub app_name: &'a str,
    pub config_hash: &'a str,
    pub project_location: &'a str
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum DeployMode {
    Local,
    Cluster
}

#[derive(Deserialize, Debug)]
pub struct ServiceConfig {
    pub deploy: DeployMode
}