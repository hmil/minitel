use crate::ServiceConfig;
use crate::Definition;
use crate::DeployMode;

use std::process::Stdio;
use std::process::Command;
use std::collections::HashMap;
use serde_yaml::Value;
use serde::{Serialize, Deserialize};
use std::io::Write;
use std::str;


#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum ConfigValue {
    // Str(&'a str),
    String(String),
    Int(i32),
    // Map(HashMap<&'a str, ConfigValue<'a>>),
    MapOwned(HashMap<String, ConfigValue>),
    List(Vec<ConfigValue>)
}

// impl<'a> From<&'a str> for ConfigValue<'a> {
//     fn from(a: &'a str) -> Self { ConfigValue::Str(a) }
// }
impl From<&'_ str> for ConfigValue {
    fn from(a: &'_ str) -> Self { ConfigValue::String(String::from(a)) }
}
impl From<String> for ConfigValue {
    fn from(a: String) -> Self { ConfigValue::String(a) }
}
impl From<i32> for ConfigValue {
    fn from(a: i32) -> Self { ConfigValue::Int(a) }
}
// impl<'a> From<HashMap<&'a str, ConfigValue<'a>>> for ConfigValue<'a> {
//     fn from(a: HashMap<&'a str, ConfigValue<'a>>) -> Self { ConfigValue::Map(a) }
// }
impl From<HashMap<String, ConfigValue>> for ConfigValue {
    fn from(a: HashMap<String, ConfigValue>) -> Self { ConfigValue::MapOwned(a) }
}
impl From<Vec<ConfigValue>> for ConfigValue {
    fn from(a: Vec<ConfigValue>) -> Self { ConfigValue::List(a) }
}

pub struct K8sServiceConfig<'a> {
    pub service_name: &'a str,
    pub app_name: &'a str,
    pub host_ip: &'a str,
    pub service_port: i32,
    pub config_hash: &'a str
}

fn tag_kube_config(config: &mut Value) -> Option<()> {
    let labels = config.get_mut("metadata")?.get_mut("labels")?;
    labels.as_mapping_mut().unwrap().insert(Value::String("minitel-app".to_string()), Value::String("minitel".to_string()));

    Some(())
}

pub fn ensure_tag_config(config: String) -> String {
    let mut buffer = Vec::new();
    let mut ser = serde_yaml::Serializer::new(&mut buffer);

    for document in serde_yaml::Deserializer::from_str(&config) {
        let mut value = Value::deserialize(document).unwrap();
        if let None = tag_kube_config(&mut value) {
            println!("Could not tag given configuration");
        }
        value.serialize(&mut ser).unwrap();
    }

    String::from(str::from_utf8(&buffer).unwrap())
}

pub fn build_routing_definition(def: &Definition, cfg: &HashMap<String, ServiceConfig>, hash: &str) -> String {
    let mut template = HashMap::new();
    template.insert("apiVersion", ConfigValue::from("v1"));
    template.insert("kind", ConfigValue::from("ConfigMap"));
    {
        let mut metadata = HashMap::new();
        metadata.insert("name".to_string(), ConfigValue::from(create_config_map_ref(&def.app_name, &hash)));
        {
            let mut labels = HashMap::new();
            labels.insert("minitel-app".to_string(), ConfigValue::from("minitel"));
            labels.insert("app".to_string(), ConfigValue::from(String::from(&def.app_name)));

            metadata.insert("labels".to_string(), ConfigValue::from(labels));
        }
        template.insert("metadata", ConfigValue::from(metadata));
    }
    {
        let mut data: HashMap<String, ConfigValue> = HashMap::new();
        for service in &def.services {
            let name = format!("{}_SERVICE", service.name.to_uppercase());
            match cfg.get(&service.name).map(|s| &s.deploy).get_or_insert(&DeployMode::Cluster) {
                DeployMode::Cluster => {
                    data.insert(name, ConfigValue::from(format!("http://{}-service", service.name)));
                }
                _ => {
                    data.insert(name, ConfigValue::from(format!("http://host.minikube.internal:{}", service.port)));
                }
            }
        }
        template.insert("data", ConfigValue::from(data));
    }
    serde_yaml::to_string(&template).unwrap()
}

pub fn build_service_cluster_definition(config: &K8sServiceConfig) -> String {
    let port_str = config.service_port.to_string();
    let image_name = format!("{}-{}", config.app_name, config.service_name);
    let image_name_version = format!("{}-{}:latest", config.app_name, config.service_name);
    let mut svc_template = HashMap::new();
    svc_template.insert("apiVersion", ConfigValue::from("v1"));
    svc_template.insert("kind", ConfigValue::from("Service"));
    {
        let mut metadata = HashMap::new();
        metadata.insert("name".to_string(), ConfigValue::from(format!("{}-service", config.service_name)));
        {
            let mut labels = HashMap::new();
            labels.insert("minitel-app".to_string(), ConfigValue::from("minitel"));
            labels.insert("app".to_string(), ConfigValue::from(config.app_name));
            labels.insert("tier".to_string(), ConfigValue::from(config.service_name));

            metadata.insert("labels".to_string(), ConfigValue::from(labels));
        }
        svc_template.insert("metadata", ConfigValue::from(metadata));
    }
    {
        let mut spec = HashMap::new();
        let mut port = HashMap::new();

        port.insert("protocol".to_string(), ConfigValue::from("TCP"));
        port.insert("port".to_string(), ConfigValue::from(80));
        port.insert("targetPort".to_string(), ConfigValue::from(config.service_port));

        spec.insert("ports".to_string(), ConfigValue::from(vec!(ConfigValue::from(port))));

        let mut selector = HashMap::new();

        selector.insert("app".to_string(), ConfigValue::from(config.app_name));
        selector.insert("tier".to_string(), ConfigValue::from(config.service_name));

        spec.insert("selector".to_string(), ConfigValue::from(selector));

        svc_template.insert("spec", ConfigValue::from(spec));
    }

    let mut document = serde_yaml::to_string(&svc_template).unwrap();

    let mut deployment_template = HashMap::new();
    deployment_template.insert("apiVersion", ConfigValue::from("apps/v1"));
    deployment_template.insert("kind", ConfigValue::from("Deployment"));
    {
        let mut metadata = HashMap::new();
        metadata.insert("name".to_string(), ConfigValue::from(config.service_name));
        {
            let mut labels = HashMap::new();
            labels.insert("minitel-app".to_string(), ConfigValue::from("minitel"));
            labels.insert("app".to_string(), ConfigValue::from(config.app_name));
            labels.insert("tier".to_string(), ConfigValue::from(config.service_name));

            metadata.insert("labels".to_string(), ConfigValue::from(labels));
        }
        deployment_template.insert("metadata", ConfigValue::from(metadata));
    }
    {
        let mut spec = HashMap::new();

        spec.insert("replicas".to_string(), ConfigValue::Int(1));

        {
            let mut selector = HashMap::new();
            let mut match_labels = HashMap::new();
            match_labels.insert("tier".to_string(), ConfigValue::from(config.service_name));
            selector.insert("matchLabels".to_string(), ConfigValue::from(match_labels));
            spec.insert("selector".to_string(), ConfigValue::from(selector));
        }
        {
            let mut template = HashMap::new();
            {
                let mut metadata = HashMap::new();
                metadata.insert("name".to_string(), ConfigValue::from(config.service_name));
                {
                    let mut labels = HashMap::new();
                    labels.insert("app".to_string(), ConfigValue::from(config.app_name));
                    labels.insert("tier".to_string(), ConfigValue::from(config.service_name));
        
                    metadata.insert("labels".to_string(), ConfigValue::from(labels));
                }
                template.insert("metadata".to_string(), ConfigValue::from(metadata));
            }
            {
                let mut spec = HashMap::new();
                let mut container = HashMap::new();

                container.insert("name".to_string(), ConfigValue::from(image_name));
                container.insert("image".to_string(), ConfigValue::from(image_name_version));
                container.insert("imagePullPolicy".to_string(), ConfigValue::from("Never"));

                {
                    let mut env = HashMap::new();
                    env.insert("name".to_string(), ConfigValue::from("PORT"));
                    env.insert("value".to_string(), ConfigValue::from(port_str));

                    container.insert("env".to_string(), ConfigValue::List(vec!(ConfigValue::from(env))));
                }
                {
                    let mut env_from = HashMap::new();
                    let mut config_map_ref = HashMap::new();
                    config_map_ref.insert("name".to_string(), ConfigValue::from(create_config_map_ref(&config.app_name, &config.config_hash)));
                    env_from.insert("configMapRef".to_string(), ConfigValue::from(config_map_ref));
                    container.insert("envFrom".to_string(), ConfigValue::from(vec!(ConfigValue::from(env_from))));
                }

                spec.insert("containers".to_string(), ConfigValue::from(vec!(ConfigValue::from(container))));

                template.insert("spec".to_string(), ConfigValue::from(spec));
            }
            spec.insert("template".to_string(), ConfigValue::from(template));
        }
        deployment_template.insert("spec", ConfigValue::from(spec));
    }

    document.push_str(&serde_yaml::to_string(&deployment_template).unwrap());

    document
}

fn create_config_map_ref(app_name: &str, config_hash: &str) -> String {
    format!("{}-routing-{}", app_name, config_hash)
}

pub fn build_service_local_definition(config: &K8sServiceConfig) -> String {
    let mut svc_template = HashMap::new();
    svc_template.insert("apiVersion", ConfigValue::from("v1"));
    svc_template.insert("kind", ConfigValue::from("Service"));
    {
        let mut metdata = HashMap::new();
        metdata.insert("name".to_string(), ConfigValue::from(format!("{}-service", config.service_name)));
        {
            let mut labels = HashMap::new();
            labels.insert("minitel-app".to_string(), ConfigValue::from("minitel"));
            labels.insert("app".to_string(), ConfigValue::from(config.app_name));
            labels.insert("tier".to_string(), ConfigValue::from(config.service_name));

            metdata.insert("labels".to_string(), ConfigValue::from(labels));
        }
        svc_template.insert("metadata", ConfigValue::from(metdata));
    }
    {
        let mut spec = HashMap::new();
        let mut port = HashMap::new();

        port.insert("protocol".to_string(), ConfigValue::from("TCP"));
        port.insert("port".to_string(), ConfigValue::from(80));

        spec.insert("ports".to_string(), ConfigValue::from(vec!(ConfigValue::from(port))));

        svc_template.insert("spec", ConfigValue::from(spec));
    }
    
    let mut document = serde_yaml::to_string(&svc_template).expect("deserialization");
    
    let mut endpoint_template = HashMap::new();
    endpoint_template.insert("apiVersion", ConfigValue::from("v1"));
    endpoint_template.insert("kind", ConfigValue::from("Endpoints"));
    {
        let mut metadata = HashMap::new();
        metadata.insert("name".to_string(), ConfigValue::from(format!("{}-service", config.service_name)));
        {
            let mut labels = HashMap::new();
            labels.insert("minitel-app".to_string(), ConfigValue::from("minitel"));
            labels.insert("app".to_string(), ConfigValue::from(config.app_name));
            labels.insert("tier".to_string(), ConfigValue::from(config.service_name));

            metadata.insert("labels".to_string(), ConfigValue::from(labels));
        }
        endpoint_template.insert("metadata", ConfigValue::from(metadata));
    }
    {
        let mut subset = HashMap::new();
        {
            let mut address = HashMap::new();
            address.insert("ip".to_string(), ConfigValue::from(config.host_ip));

            subset.insert("addresses".to_string(), ConfigValue::from(vec!(ConfigValue::from(address))));
        }
        {
            let mut ports = HashMap::new();
            ports.insert("port".to_string(), ConfigValue::from(config.service_port));

            subset.insert("ports".to_string(), ConfigValue::from(vec!(ConfigValue::from(ports))));
        }

        endpoint_template.insert("subsets", ConfigValue::from(vec!(ConfigValue::from(subset))));
    }

    document.push_str(&serde_yaml::to_string(&endpoint_template).expect("Failed to serialize"));

    document
}

pub fn kubectl_apply(config: &str) {
    let mut cmd = Command::new("kubectl")
        .arg("apply")
        .arg("-f")
        .arg("-")
        .stdin(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    cmd.stdin.as_mut().unwrap().write_all(config.as_bytes()).unwrap();

    let ecode = cmd.wait()
        .expect("failed to wait on child");

    assert!(ecode.success());
}

pub fn kubectl_delete_all() {
    let status = Command::new("kubectl")
        .arg("delete")
        .arg("all,ingress,configmap")
        .arg("-l")
        .arg("minitel-app=minitel")
        .arg("--wait=true")
        .spawn()
        .expect("Failed to execute command")
        .wait()
        .expect("Command failed");

    assert!(status.success());
}