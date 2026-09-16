#!/usr/bin/env python3
"""
502Drive Tray Controller
A lightweight desktop indicator for controlling 502Drive Telegram Bot service on GNOME/KDE/Linux.
Clean, professional GTK interface without emojis, using native symbolic icons.
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
APP_NAME = "502Drive"
APP_ID = "502drive"
ICON_ACTIVE = str(Path.home() / ".local/share/icons/hicolor/scalable/apps/502drive-symbolic.svg")
ICON_INACTIVE = str(Path.home() / ".local/share/icons/hicolor/scalable/apps/502drive-inactive-symbolic.svg")
CONFIG_PATH = Path.home() / ".config" / "gdclone-bot" / "config.toml"


def run_cmd(cmd):
    try:
        res = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=10)
        return res.returncode == 0, res.stdout.strip(), res.stderr.strip()
    except Exception as e:
        return False, "", str(e)


def notify(title, message, icon="502drive-symbolic"):
    subprocess.Popen(["notify-send", "-a", APP_NAME, "-i", icon, title, message])


def make_menu_item(label_text, icon_name=None, callback=None):
    item = Gtk.MenuItem()
    box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
    img = None
    if icon_name:
        img = Gtk.Image.new_from_icon_name(icon_name, Gtk.IconSize.MENU)
        box.pack_start(img, False, False, 0)
    lbl = Gtk.Label(label=label_text, xalign=0)
    box.pack_start(lbl, True, True, 0)
    item.add(box)
    if callback:
        item.connect("activate", callback)
    return item, lbl, img


class DriveTray:
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
        self.status_item, self.status_lbl, self.status_img = make_menu_item(
            "502Drive: Đang kiểm tra...", "network-idle-symbolic"
        )
        self.status_item.set_sensitive(False)
        self.menu.append(self.status_item)

        self.menu.append(Gtk.SeparatorMenuItem())

        # 2. Open Telegram
        tg_item, _, _ = make_menu_item(
            "Mở Telegram Bot (@p502Drive_bot)", "send-to-symbolic", self.open_telegram
        )
        self.menu.append(tg_item)

        self.menu.append(Gtk.SeparatorMenuItem())

        # 3. Toggle Start/Stop
        self.toggle_item, self.toggle_lbl, self.toggle_img = make_menu_item(
            "Tạm dừng dịch vụ", "media-playback-pause-symbolic", self.toggle_service
        )
        self.menu.append(self.toggle_item)

        # 4. Restart
        restart_item, _, _ = make_menu_item(
            "Khởi động lại dịch vụ", "view-refresh-symbolic", self.restart_service
        )
        self.menu.append(restart_item)

        # 5. Doctor
        doctor_item, _, _ = make_menu_item(
            "Kiểm tra hệ thống (Doctor)", "system-run-symbolic", self.run_doctor
        )
        self.menu.append(doctor_item)

        # 6. View Logs
        logs_item, _, _ = make_menu_item(
            "Xem nhật ký hoạt động (Logs)", "utilities-terminal-symbolic", self.view_logs
        )
        self.menu.append(logs_item)

        # 7. Edit Config
        config_item, _, _ = make_menu_item(
            "Tệp cấu hình (config.toml)", "document-properties-symbolic", self.edit_config
        )
        self.menu.append(config_item)

        self.menu.append(Gtk.SeparatorMenuItem())

        # 8. Quit Tray
        quit_item, _, _ = make_menu_item(
            "Đóng khay hệ thống", "application-exit-symbolic", self.quit_app
        )
        self.menu.append(quit_item)

        self.menu.show_all()

    def update_status(self):
        active = self.is_service_active()
        if active != self.last_status:
            self.last_status = active
            if active:
                self.indicator.set_icon_full(ICON_ACTIVE, "Active")
                self.status_lbl.set_text("502Drive: Đang hoạt động")
                self.status_img.set_from_icon_name("emblem-default-symbolic", Gtk.IconSize.MENU)
                self.toggle_lbl.set_text("Tạm dừng dịch vụ")
                self.toggle_img.set_from_icon_name("media-playback-pause-symbolic", Gtk.IconSize.MENU)
            else:
                self.indicator.set_icon_full(ICON_INACTIVE, "Inactive")
                self.status_lbl.set_text("502Drive: Đã dừng")
                self.status_img.set_from_icon_name("process-stop-symbolic", Gtk.IconSize.MENU)
                self.toggle_lbl.set_text("Khởi động dịch vụ")
                self.toggle_img.set_from_icon_name("media-playback-start-symbolic", Gtk.IconSize.MENU)
        return True

    def open_telegram(self, _):
        subprocess.Popen(["xdg-open", "tg://resolve?domain=p502Drive_bot"])

    def toggle_service(self, _):
        if self.is_service_active():
            run_cmd(f"systemctl --user stop {SERVICE_NAME}")
            notify("502Drive", "Đã dừng dịch vụ bot.", ICON_INACTIVE)
        else:
            run_cmd(f"systemctl --user start {SERVICE_NAME}")
            notify("502Drive", "Đã khởi động dịch vụ bot.", ICON_ACTIVE)
        self.update_status()

    def restart_service(self, _):
        run_cmd(f"systemctl --user restart {SERVICE_NAME}")
        notify("502Drive", "Đã khởi động lại dịch vụ bot.", ICON_ACTIVE)
        self.update_status()

    def run_doctor(self, _):
        ok, out, err = run_cmd("gdclone-bot doctor")
        msg = out if ok else (err or out or "Lỗi khi kiểm tra doctor")
        subprocess.Popen([
            "zenity", "--info",
            "--title=502Drive Doctor Report",
            "--text=" + msg,
            "--width=480", "--height=320"
        ])

    def view_logs(self, _):
        term_cmd = None
        for t in ["ptyxis", "gnome-terminal", "kgx", "xterm"]:
            if subprocess.run(f"which {t}", shell=True, capture_output=True).returncode == 0:
                term_cmd = t
                break
        if term_cmd == "ptyxis":
            subprocess.Popen(["ptyxis", "--title", "502Drive Live Logs", "--", "journalctl", "--user", "-u", SERVICE_NAME, "-f"])
        elif term_cmd == "gnome-terminal":
            subprocess.Popen(["gnome-terminal", "--title", "502Drive Live Logs", "--", "journalctl", "--user", "-u", SERVICE_NAME, "-f"])
        elif term_cmd:
            subprocess.Popen([term_cmd, "-e", f"journalctl --user -u {SERVICE_NAME} -f"])
        else:
            notify("502Drive Logs", "Mở terminal và gõ: journalctl --user -u gdclone-bot -f")

    def edit_config(self, _):
        if CONFIG_PATH.exists():
            subprocess.Popen(["xdg-open", str(CONFIG_PATH)])
        else:
            notify("502Drive", f"Không tìm thấy file {CONFIG_PATH}")

    def quit_app(self, _):
        Gtk.main_quit()


def main():
    signal.signal(signal.SIGINT, signal.SIG_DFL)
    _app = DriveTray()
    Gtk.main()


if __name__ == "__main__":
    main()
