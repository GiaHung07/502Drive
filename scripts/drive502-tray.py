#!/usr/bin/env python3
"""
Drive502 Tray Controller
A lightweight desktop indicator for controlling Drive502 Telegram Bot service on GNOME/KDE/Linux.
"""

import os
import signal
import subprocess
import sys
from pathlib import Path

import gi
gi.require_version("Gtk", "3.0")
gi.require_version("AyatanaAppIndicator3", "0.1")
from gi.repository import AyatanaAppIndicator3, GLib, Gtk

SERVICE_NAME = "gdclone-bot"
APP_NAME = "Drive502 Bot"
APP_ID = "drive502"
ICON_ACTIVE = "drive502"
ICON_INACTIVE = "drive502-inactive"
CONFIG_PATH = Path.home() / ".config" / "gdclone-bot" / "config.toml"


def run_cmd(cmd):
    try:
        res = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=10)
        return res.returncode == 0, res.stdout.strip(), res.stderr.strip()
    except Exception as e:
        return False, "", str(e)


def notify(title, message, icon="drive502"):
    subprocess.Popen(["notify-send", "-a", APP_NAME, "-i", icon, title, message])


class Drive502Tray:
    def __init__(self):
        self.indicator = AyatanaAppIndicator3.Indicator.new(
            APP_ID,
            ICON_ACTIVE,
            AyatanaAppIndicator3.IndicatorCategory.APPLICATION_STATUS,
        )
        self.indicator.set_status(AyatanaAppIndicator3.IndicatorStatus.ACTIVE)
        self.indicator.set_title(APP_NAME)

        self.menu = Gtk.Menu()
        self.build_menu()
        self.indicator.set_menu(self.menu)

        self.last_status = None
        self.update_status()

        # Check service status every 3 seconds
        GLib.timeout_add_seconds(3, self.update_status)

    def is_service_active(self):
        ok, out, _ = run_cmd(f"systemctl --user is-active {SERVICE_NAME}")
        return ok and out == "active"

    def build_menu(self):
        # 1. Header / Status item
        self.status_item = Gtk.MenuItem(label="Drive502: Đang kiểm tra...")
        self.status_item.set_sensitive(False)
        self.menu.append(self.status_item)

        self.menu.append(Gtk.SeparatorMenuItem())

        # 2. Open Telegram
        tg_item = Gtk.MenuItem(label="✈️ Mở Telegram Bot (@Drive502_Bot)")
        tg_item.connect("activate", self.open_telegram)
        self.menu.append(tg_item)

        self.menu.append(Gtk.SeparatorMenuItem())

        # 3. Toggle Start/Stop
        self.toggle_item = Gtk.MenuItem(label="⏸️ Tạm dừng Bot")
        self.toggle_item.connect("activate", self.toggle_service)
        self.menu.append(self.toggle_item)

        # 4. Restart
        restart_item = Gtk.MenuItem(label="🔄 Khởi động lại Bot")
        restart_item.connect("activate", self.restart_service)
        self.menu.append(restart_item)

        # 5. Doctor
        doctor_item = Gtk.MenuItem(label="🩺 Kiểm tra hệ thống (Doctor)")
        doctor_item.connect("activate", self.run_doctor)
        self.menu.append(doctor_item)

        # 6. View Logs
        logs_item = Gtk.MenuItem(label="📋 Xem nhật ký (Live Logs)")
        logs_item.connect("activate", self.view_logs)
        self.menu.append(logs_item)

        # 7. Edit Config
        config_item = Gtk.MenuItem(label="⚙️ Cấu hình (config.toml)")
        config_item.connect("activate", self.edit_config)
        self.menu.append(config_item)

        self.menu.append(Gtk.SeparatorMenuItem())

        # 8. Quit Tray
        quit_item = Gtk.MenuItem(label="🚪 Đóng Tray Icon")
        quit_item.connect("activate", self.quit_app)
        self.menu.append(quit_item)

        self.menu.show_all()

    def update_status(self):
        active = self.is_service_active()
        if active != self.last_status:
            self.last_status = active
            if active:
                self.indicator.set_icon_full(ICON_ACTIVE, "Active")
                self.status_item.set_label("🟢 Drive502: Đang chạy (Active)")
                self.toggle_item.set_label("⏸️ Tạm dừng Bot")
            else:
                self.indicator.set_icon_full(ICON_INACTIVE, "Inactive")
                self.status_item.set_label("🔴 Drive502: Đã dừng (Inactive)")
                self.toggle_item.set_label("▶️ Bật Bot")
        return True

    def open_telegram(self, _):
        # Try native telegram link first, fallback to web
        subprocess.Popen(["xdg-open", "tg://resolve?domain=Drive502_Bot"])

    def toggle_service(self, _):
        if self.is_service_active():
            run_cmd(f"systemctl --user stop {SERVICE_NAME}")
            notify("Drive502", "Đã dừng dịch vụ bot.", ICON_INACTIVE)
        else:
            run_cmd(f"systemctl --user start {SERVICE_NAME}")
            notify("Drive502", "Đã khởi động dịch vụ bot.", ICON_ACTIVE)
        self.update_status()

    def restart_service(self, _):
        run_cmd(f"systemctl --user restart {SERVICE_NAME}")
        notify("Drive502", "Đã khởi động lại dịch vụ bot.", ICON_ACTIVE)
        self.update_status()

    def run_doctor(self, _):
        ok, out, err = run_cmd("gdclone-bot doctor")
        msg = out if ok else (err or out or "Lỗi khi kiểm tra doctor")
        # Display using zenity info dialog if available
        subprocess.Popen([
            "zenity", "--info",
            "--title=Drive502 Doctor Check",
            "--text=" + msg,
            "--width=480", "--height=320",
            "--icon-name=drive502"
        ])

    def view_logs(self, _):
        # Try ptyxis or gnome-terminal
        term_cmd = None
        for t in ["ptyxis", "gnome-terminal", "kgx", "xterm"]:
            if subprocess.run(f"which {t}", shell=True, capture_output=True).returncode == 0:
                term_cmd = t
                break
        if term_cmd == "ptyxis":
            subprocess.Popen(["ptyxis", "--title", "Drive502 Live Logs", "--", "journalctl", "--user", "-u", SERVICE_NAME, "-f"])
        elif term_cmd == "gnome-terminal":
            subprocess.Popen(["gnome-terminal", "--title", "Drive502 Live Logs", "--", "journalctl", "--user", "-u", SERVICE_NAME, "-f"])
        elif term_cmd:
            subprocess.Popen([term_cmd, "-e", f"journalctl --user -u {SERVICE_NAME} -f"])
        else:
            notify("Drive502 Logs", "Mở terminal và gõ: journalctl --user -u gdclone-bot -f")

    def edit_config(self, _):
        if CONFIG_PATH.exists():
            subprocess.Popen(["xdg-open", str(CONFIG_PATH)])
        else:
            notify("Drive502", f"Không tìm thấy file {CONFIG_PATH}")

    def quit_app(self, _):
        Gtk.main_quit()


def main():
    # Handle Ctrl+C cleanly
    signal.signal(signal.SIGINT, signal.SIG_DFL)
    _app = Drive502Tray()
    Gtk.main()


if __name__ == "__main__":
    main()
