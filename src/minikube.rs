
use std::process::Command;
use std::str;

pub fn get_host_ip() -> String {
    let output = Command::new("sh")
        .arg("-c")
        .arg("minikube ssh cat /etc/hosts | grep host.minikube.internal | cut -f1")
        .output()
        .expect("Failed to execute command");

    let result = str::from_utf8(output.stdout.as_slice()).expect("");
    println!("stderr is: {}", str::from_utf8(output.stderr.as_slice()).expect(""));
    println!("Host ip is: {}", result);

    if !output.status.success() || result.is_empty() {
        panic!("Could not obtain host ip address. Is minikube running?");
    }

    return String::from(result.trim());
}

pub fn get_minikube_ip() -> String {
    let output = Command::new("minikube")
        .arg("ip")
        .output()
        .expect("Failed to execute command");

    let result = str::from_utf8(output.stdout.as_slice()).expect("");
    println!("stderr is: {}", str::from_utf8(output.stderr.as_slice()).expect(""));
    println!("Minikube ip is: {}", result);

    if !output.status.success() || result.is_empty() {
        panic!("Could not obtain host ip address. Is minikube running?");
    }

    return String::from(result.trim());
}