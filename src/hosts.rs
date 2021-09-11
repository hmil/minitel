use crate::Definition;
use crate::GlobalConfig;

use std::io::BufWriter;
use std::io::Write;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

/// Checks if hosts file is up-to-date.
/// 
/// Returns true if it is, false if it's not
pub fn check_etc_hosts(config: &GlobalConfig, def: &Definition) -> io::Result<bool> {
    let file = File::open("/etc/hosts")?;

    // Remove entry if there is already one
    for line in BufReader::new(file).lines() {
        if let Ok(l) = line {
            if l.contains(&def.hostname) {
                if l.contains(&config.minikube_ip) {
                    return Ok(true);
                } else {
                    return Ok(false);
                }
            }
        }
    }

    Ok(false)
}

pub fn patch_etc_hosts(config: &GlobalConfig, def: &Definition) -> io::Result<()> {
    let file = File::open("/etc/hosts")?;
    
    let mut acc: Vec<String> = Vec::new();

    // Remove entry if there is already one
    for line in BufReader::new(file).lines() {
        if let Ok(l) = line {
            if !l.contains(&def.hostname) {
                acc.push(l);
            }
        }
    }

    let entry = format!("{} {} # minitel application", &config.minikube_ip, &def.hostname);
    acc.push(entry);

    let file = File::create("/etc/hosts")?;
    let mut f = BufWriter::new(file);
    let ln = b"\n";

    for line in acc {
        f.write_all(&line.into_bytes())?;
        f.write_all(ln)?;
    }

    f.flush()?;

    Ok(())
}