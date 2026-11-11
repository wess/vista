#!/bin/sh
# Install Vista Tools in a Linux guest. Run as root from Vista's tools share:
#   sudo mount -t virtiofs vistatools /mnt && sudo /mnt/install.sh
set -e
here="$(cd "$(dirname "$0")" && pwd)"
if [ "$(id -u)" != "0" ]; then
  echo "Run this as root: sudo $0" >&2
  exit 1
fi

install -d /usr/local/bin
cp "$here/linux/vistatools" /usr/local/bin/vistatools.new
chmod 755 /usr/local/bin/vistatools.new
mv -f /usr/local/bin/vistatools.new /usr/local/bin/vistatools
modprobe vmw_vsock_virtio_transport 2>/dev/null || true

if command -v systemctl >/dev/null 2>&1 && [ -d /run/systemd/system ]; then
  cat > /etc/systemd/system/vistatools.service <<'UNIT'
[Unit]
Description=Vista Tools
After=local-fs.target

[Service]
ExecStartPre=-/sbin/modprobe vmw_vsock_virtio_transport
ExecStart=/usr/local/bin/vistatools
Restart=always
RestartSec=2

[Install]
WantedBy=multi-user.target
UNIT
  systemctl daemon-reload
  systemctl enable vistatools >/dev/null 2>&1
  systemctl restart vistatools
elif command -v rc-update >/dev/null 2>&1; then
  cat > /etc/init.d/vistatools <<'RC'
#!/sbin/openrc-run
description="Vista Tools"
command=/usr/local/bin/vistatools
command_background=true
pidfile=/run/vistatools.pid
output_log=/var/log/vistatools.log
error_log=/var/log/vistatools.log
start_pre() { modprobe vmw_vsock_virtio_transport 2>/dev/null || true; }
RC
  chmod 755 /etc/init.d/vistatools
  rc-update add vistatools default >/dev/null 2>&1
  rc-service vistatools restart >/dev/null 2>&1 || rc-service vistatools start
else
  nohup /usr/local/bin/vistatools >/var/log/vistatools.log 2>&1 &
  echo "No init system found: Vista Tools runs until the next reboot."
fi

# Mount the folders Vista shares at /mnt/vista on every boot.
mkdir -p /mnt/vista
if ! grep -q '^vista ' /etc/fstab 2>/dev/null; then
  echo 'vista /mnt/vista virtiofs defaults,nofail 0 0' >> /etc/fstab
fi
mount /mnt/vista 2>/dev/null || true

echo "Vista Tools is installed and running."
