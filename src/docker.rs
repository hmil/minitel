
use std::process::Command;

pub fn build_service_image(project_location: &str, app_name: &str, service_name: &str) {
    let service_image_name = format!("{}-{}", app_name, service_name);
    let service_directory = format!("{}/services/{}", project_location, service_name);

    let status = Command::new("docker")
        .arg("build")
        .arg("-t")
        .arg(&service_image_name)
        .arg(&service_directory)
        .spawn()
        .expect("Failed to execute command")
        .wait()
        .expect("Command failed");

    if !status.success() {
        println!("Failed to build docker image");
    }
}