#!/bin/bash
sudo rm /etc/systemd/system/chrome-driver.service

cat > /etc/systemd/system/chrome-driver.service <<- EOF
[Unit]
Description=Chrome Driver

[Service]
WorkingDirectory=/usr/bin
ExecStart=/usr/bin/chromedriver --port=3000
Restart=always
RestartSec=10
SyslogIdentifier=chromedriver
User=root

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl start chrome-driver.service
./tetsuki-actix