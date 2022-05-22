#![allow(unreachable_code)]

use std::{time::Duration, error::Error, collections::HashMap, thread};
use dirs::{config_dir};
use zbus::{blocking::Connection, blocking::Proxy, dbus_proxy};
use zvariant::Value;

use serde_derive::{Serialize, Deserialize};


#[dbus_proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: &HashMap<&str, &Value<'_>>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;
}

#[derive(Debug, Serialize, Deserialize)]
struct MyConfig {
    controllers: Vec<String>,
    display_controller_connected: bool,
    display_controller_charging: bool,
    display_controller_low_battery: bool
}

impl ::std::default::Default for MyConfig {
    fn default() -> Self { Self { 
        controllers: [].to_vec(), 
        display_controller_connected: true,
        display_controller_charging: false,
        display_controller_low_battery: true
    } }
}


fn main() -> Result<(), Box<dyn Error>> {
    let cfg: MyConfig = confy::load("conbatt-rs")?;
    
    let mut config_path_str = String::from("");
    match config_dir() {
        Some(mut path) => {
            path.push("conbatt-rs");
            path.push("controller.png");
            match path.to_str() {
                Some(path_str) => config_path_str = path_str.to_owned(),
                None => ()
            }
        },
        None => {
            println!("Error: no config path?");
            return Ok(());
        }
    };

    loop {
        println!("Searching for controllers");
        let conn = Connection::system()?;
        let nots = Connection::session()?;
        let proxy = NotificationsProxyBlocking::new(&nots)?;
        let resp = conn.call_method(
            Some("org.freedesktop.UPower"),
            "/org/freedesktop/UPower",
            Some("org.freedesktop.UPower"),
            "EnumerateDevices",
            &(),
        )?;
        let mut lastbattery: u32;
        let mut is_now_charging = false;
        let mut is_low_battery = false;

        let devices: Vec<zvariant::ObjectPath> = resp.body()?;
        for device_path in devices {
            let device = Proxy::new(
                &conn,
                "org.freedesktop.UPower",
                device_path,
                "org.freedesktop.UPower.Device"
                
            )?;

            
            let model: String = device.get_property("Model")?;
            let serial: String = device.get_property("Serial")?;
            if cfg.controllers.contains(&model) || cfg.controllers.contains(&serial) {
                println!("Found controller");
                 if cfg.display_controller_connected {
                    let percentage: f64 = device.get_property("Percentage")?;
                    proxy.notify(
                        "ConBattRS",
                        0,
                        &format!("file://{}", config_path_str),
                        "Controller connected.",
                        &format!("Device: '{}'. Currently at {}%", model, percentage),
                        &[],
                        &HashMap::new(),
                        5000,
                    )?;
                }
                let battery: u32 = device.get_property("BatteryLevel")?;
                lastbattery = battery;
                loop {
                    //let err: Result<(), zbus::Error> = device.call("Refresh", &());
                    let err: Result<(), zbus::Error> = device.call("GetHistory", &());
                    match err {
                        Err(error) => match error {
                            zbus::Error::MethodError(errorname, _, _message) => {
                                // Probablemente si no existe el metodo no esta conectado
                                if errorname == "org.freedesktop.DBus.Error.UnknownMethod" {
                                    break
                                }
                            },
                            _ => ()
                        },
                        Ok(_) => ()
                    };
                    

                    let battery: u32 = device.get_property("BatteryLevel")?;
                    let is_charging: u32 = device.get_property("State")?;

                    

                    if lastbattery < battery {
                        let percentage: f64 = device.get_property("Percentage")?;
                        println!("Battery {}%", percentage);
                        proxy.notify(
                            "ConBattRS",
                            0,
                            &format!("file://{}", config_path_str),
                            "The controller battery has increased.",
                            &format!("Currently at {}%", percentage),
                            &[],
                            &HashMap::new(),
                            5000,
                        )?;
                        lastbattery = battery;
                    }
                    if battery < 6 && !is_low_battery && cfg.display_controller_low_battery {
                        let percentage: f64 = device.get_property("Percentage")?;
                        println!("Battery {}%", percentage);
                        proxy.notify(
                            "ConBattRS",
                            0,
                            &format!("file://{}", config_path_str),
                            "Low battery.",
                            &format!("Currently at {}%", percentage),
                            &[],
                            &HashMap::new(),
                            5000,
                        )?;
                        is_low_battery = true;
                        lastbattery = battery;
                    }
                    if battery >= 6 && is_low_battery && cfg.display_controller_low_battery {
                        is_low_battery = false;
                        lastbattery = battery;
                    }

                    if is_charging == 1 && !is_now_charging && cfg.display_controller_charging {
                        is_now_charging = true;
                        println!("Now charging...");
                        let percentage: f64 = device.get_property("Percentage")?;
                        proxy.notify(
                            "ConBattRS",
                            0,
                            &format!("file://{}", config_path_str),
                            "Now charging...",
                            &format!("Currently at {}%", percentage),
                            &[],
                            &HashMap::new(),
                            5000,
                        )?;
                    }
                    if is_charging == 0 && is_now_charging && cfg.display_controller_charging {
                        is_now_charging = false;
                        lastbattery = battery;
                    }
                    thread::sleep(Duration::from_millis(3000));
                }
            }
        }
        thread::sleep(Duration::from_millis(2500));    
    }
    Ok(())
}