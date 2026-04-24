use std::{fs, path::PathBuf, sync::Arc};

use camino::Utf8PathBuf;
use crow_driver::{Driver, DriverConfig};

fn main() {
    let mut driver = Driver::new(DriverConfig::new(
        Utf8PathBuf::from("C:\\Users\\vyacheslav\\crow\\test\\src"),
        Utf8PathBuf::from("C:\\Users\\vyacheslav\\crow\\test\\target"),
    ));
    driver.perform_compilation();
}
