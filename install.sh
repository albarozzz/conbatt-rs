#!/bin/bash

if [ "$(id -u)" -ne 1000 ]
  then echo "Do not run as root please"
  exit
fi

killall conbatt-rs
cargo build --release
sudo cp ./target/release/conbatt-rs /usr/bin/
sudo cp ./conbatt.service /etc/systemd/user/
systemctl enable --user conbatt.service
systemctl start --user conbatt.service

mkdir -p $HOME/.config/conbatt-rs
cp ./controller.png $HOME/.config/conbatt-rs/
