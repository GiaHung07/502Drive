#!/usr/bin/env python3
"""
502Drive Tray Controller
A polished desktop indicator for managing 502Drive Telegram Bot service on GNOME/Linux.
Styled with native submenus and symbolic icons, adhering to 502Print design standards (Zero Emojis).
"""

import os
import signal
import sqlite3
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
DB_PATH = Path.home() / ".local" / "share" / "gdclone-bot" / "state.db"
REPORTS_DIR = Path.home() / ".local" / "share" / "gdclone-bot" / "reports"
AUTOSTART_DESKTOP = Path.home() / ".config" / "autostart" / "502drive.desktop"
INSTALLED_DESKTOP = Path.home() / ".local" / "share" / "applications" / "502drive.desktop"


def run_cmd(cmd):
    try:
        res = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=10)
        return res.returncode == 0, res.stdout.strip(), res.stderr.strip()
    except Exception as e:
        return False, "", str(e)


def notify(title, message, icon="502drive-symbolic"):
    subprocess.Popen(["notify-send", "-a", APP_NAME, "-i", icon, title, message])


def get_terminal_cmd():
    for t in ["ptyxis", "gnome-terminal", "kgx", "xterm"]:
        if subprocess.run(f"which {t}", shell=True, capture_output=True).returncode == 0:
            return t
    return None


def open_in_terminal(title, command):
    term = get_terminal_cmd()
    if term == "ptyxis":
        subprocess.Popen(["ptyxis", "--title", title, "--", "bash", "-c", f"{command}; echo; read -p 'Nhấn Enter để đóng...'"])
    elif term == "gnome-terminal":
        subprocess.Popen(["gnome-terminal", "--title", title, "--", "bash", "-c", f"{command}; echo; read -p 'Nhấn Enter để đóng...'"])
    elif term:
        subprocess.Popen([term, "-e", f"bash -c \"{command}; echo; read -p 'Nhấn Enter để đóng...'\""])
    else:
        notify(title, f"Chạy lệnh: {command}")


def make_item(label, icon_name=None, callback=None, sensitive=True):
    if icon_name:
        item = Gtk.ImageMenuItem.new_with_label(label)
        img = Gtk.Image.new_from_icon_name(icon_name, Gtk.IconSize.MENU)
        item.set_image(img)
        item.set_always_show_image(True)
    else:
        item = Gtk.MenuItem.new_with_label(label)
    item.set_sensitive(sensitive)
    if callback:
        item.connect("activate", callback)
    return item


def make_submenu(label, icon_name=None):
    if icon_name:
        item = Gtk.ImageMenuItem.new_with_label(label)
        img = Gtk.Image.new_from_icon_name(icon_name, Gtk.IconSize.MENU)
        item.set_image(img)
        item.set_always_show_image(True)
    else:
        item = Gtk.MenuItem.new_with_label(label)
    sub = Gtk.Menu()
    item.set_submenu(sub)
    return item, sub


def make_check_item(label, is_active=False, callback=None):
    item = Gtk.CheckMenuItem.new_with_label(label)
    item.set_active(is_active)
    if callback:
        item.connect("toggled", callback)
    return item


class DriveTray:
    def __init__(self):
        self.indicator = AyatanaAppIndicator3.Indicator.new(
            APP_ID,
            ICON_ACTIVE,
            AyatanaAppIndicator3.IndicatorCategory.APPLICATION_STATUS,
        )
        self.indicator.set_status(AyatanaAppIndicator3.IndicatorStatus.ACTIVE)
        self.indicator.set_title(APP_NAME)

        self.last_status = None
        self.building_menu = False

        self.menu = Gtk.Menu()
        self.build_menu()
        self.indicator.set_menu(self.menu)

        self.update_status()
        GLib.timeout_add_seconds(3, self.update_status)

    def is_service_active(self):
        ok, out, _ = run_cmd(f"systemctl --user is-active {SERVICE_NAME}")
        return ok and out == "active"

    def get_google_account_info(self):
        if not DB_PATH.exists():
            return None, "Chưa có CSDL"
        try:
            conn = sqlite3.connect(str(DB_PATH))
            cur = conn.cursor()
            cur.execute("SELECT email, status FROM google_accounts LIMIT 1")
            row = cur.fetchone()
            conn.close()
            if row and row[0]:
                return row[0], row[1]
            return None, "Chưa đăng nhập"
        except Exception:
            return None, "Chưa kết nối"

    def get_config_language(self):
        if CONFIG_PATH.exists():
            try:
                with open(CONFIG_PATH, "r", encoding="utf-8") as f:
                    for line in f:
                        s = line.strip()
                        if s.startswith("language"):
                            if '"en"' in s or "'en'" in s:
                                return "en"
                            return "vi"
            except Exception:
                pass
        return "vi"

    def is_autostart_enabled(self):
        ok, out, _ = run_cmd(f"systemctl --user is-enabled {SERVICE_NAME}")
        service_enabled = ok and out == "enabled"
        desktop_exists = AUTOSTART_DESKTOP.exists()
        return service_enabled or desktop_exists

    def build_menu(self):
        self.building_menu = True
        # Clear old items
        for child in self.menu.get_children():
            self.menu.remove(child)

        is_active = self.is_service_active()
        google_email, google_status = self.get_google_account_info()
        current_lang = self.get_config_language()
        autostart_on = self.is_autostart_enabled()

        # 1. Primary Action: Open Telegram Bot
        tg_item = make_item(
            "Mở Telegram Bot (@Drive502_Bot)",
            icon_name="send-to-symbolic",
            callback=self.open_telegram,
        )
        self.menu.append(tg_item)

        self.menu.append(Gtk.SeparatorMenuItem())

        # 2. Google Account Submenu
        google_item, google_sub = make_submenu("Tài khoản Google", icon_name="avatar-default-symbolic")
        
        acc_label = f"Tài khoản: {google_email}" if google_email else "Tài khoản: Chưa kết nối"
        acc_status_item = make_item(acc_label, icon_name="emblem-default-symbolic" if google_email else "dialog-warning-symbolic", sensitive=False)
        google_sub.append(acc_status_item)

        google_sub.append(Gtk.SeparatorMenuItem())

        login_item = make_item(
            "Đăng nhập / Đổi tài khoản...",
            icon_name="system-switch-user-symbolic",
            callback=self.login_google,
        )
        google_sub.append(login_item)

        if google_email:
            revoke_item = make_item(
                "Huỷ kết nối tài khoản này",
                icon_name="user-trash-symbolic",
                callback=self.revoke_google,
            )
            google_sub.append(revoke_item)

        gdrive_web_item = make_item(
            "Mở Google Drive trên Web...",
            icon_name="web-browser-symbolic",
            callback=lambda _: subprocess.Popen(["xdg-open", "https://drive.google.com"]),
        )
        google_sub.append(gdrive_web_item)

        self.menu.append(google_item)

        # 3. Bot Service Submenu
        service_item, service_sub = make_submenu("Dịch vụ Bot", icon_name="network-workgroup-symbolic")

        status_text = "Trạng thái: Đang hoạt động" if is_active else "Trạng thái: Đã tạm dừng"
        status_icon = "emblem-default-symbolic" if is_active else "process-stop-symbolic"
        svc_status_item = make_item(status_text, icon_name=status_icon, sensitive=False)
        service_sub.append(svc_status_item)

        service_sub.append(Gtk.SeparatorMenuItem())

        if is_active:
            toggle_item = make_item(
                "Tạm dừng dịch vụ",
                icon_name="media-playback-pause-symbolic",
                callback=self.stop_service,
            )
        else:
            toggle_item = make_item(
                "Khởi động dịch vụ",
                icon_name="media-playback-start-symbolic",
                callback=self.start_service,
            )
        service_sub.append(toggle_item)

        restart_item = make_item(
            "Khởi động lại dịch vụ",
            icon_name="view-refresh-symbolic",
            callback=self.restart_service,
        )
        service_sub.append(restart_item)

        self.menu.append(service_item)

        # 4. Tools & Data Submenu
        tools_item, tools_sub = make_submenu("Quản trị & Dữ liệu", icon_name="folder-documents-symbolic")

        doctor_item = make_item(
            "Kiểm tra hệ thống (Doctor)...",
            icon_name="dialog-information-symbolic",
            callback=self.run_doctor,
        )
        tools_sub.append(doctor_item)

        reports_item = make_item(
            "Mở thư mục Báo cáo (Reports)...",
            icon_name="folder-symbolic",
            callback=self.open_reports,
        )
        tools_sub.append(reports_item)

        backup_item = make_item(
            "Sao lưu dữ liệu (Backup)...",
            icon_name="document-save-symbolic",
            callback=self.run_backup,
        )
        tools_sub.append(backup_item)

        logs_item = make_item(
            "Xem nhật ký trực tiếp (Live Logs)...",
            icon_name="utilities-terminal-symbolic",
            callback=self.view_logs,
        )
        tools_sub.append(logs_item)

        config_item = make_item(
            "Tệp cấu hình (config.toml)...",
            icon_name="document-properties-symbolic",
            callback=self.edit_config,
        )
        tools_sub.append(config_item)

        self.menu.append(tools_item)

        self.menu.append(Gtk.SeparatorMenuItem())

        # 5. Autostart Submenu
        autostart_item, autostart_sub = make_submenu("Khởi động cùng hệ thống", icon_name="system-run-symbolic")

        as_on = make_check_item("Bật khởi động cùng hệ thống", is_active=autostart_on, callback=lambda w: self.set_autostart(w, True))
        as_off = make_check_item("Tắt khởi động cùng hệ thống", is_active=not autostart_on, callback=lambda w: self.set_autostart(w, False))
        autostart_sub.append(as_on)
        autostart_sub.append(as_off)
        self.menu.append(autostart_item)

        # 6. Language Submenu
        lang_item, lang_sub = make_submenu("Ngôn ngữ / Language", icon_name="preferences-desktop-locale-symbolic")
        
        lang_vi = make_check_item("Tiếng Việt", is_active=(current_lang == "vi"), callback=lambda w: self.set_lang(w, "vi"))
        lang_en = make_check_item("English", is_active=(current_lang == "en"), callback=lambda w: self.set_lang(w, "en"))
        lang_sub.append(lang_vi)
        lang_sub.append(lang_en)
        self.menu.append(lang_item)

        self.menu.append(Gtk.SeparatorMenuItem())

        # 7. Quit Tray Applet
        quit_item = make_item(
            "Thoát 502Drive Tray",
            icon_name="application-exit-symbolic",
            callback=self.quit_app,
        )
        self.menu.append(quit_item)

        self.menu.show_all()
        self.building_menu = False

    def update_status(self):
        active = self.is_service_active()
        if active != self.last_status:
            self.last_status = active
            if active:
                self.indicator.set_icon_full(ICON_ACTIVE, "Active")
            else:
                self.indicator.set_icon_full(ICON_INACTIVE, "Inactive")
            self.build_menu()
        return True

    def open_telegram(self, _):
        subprocess.Popen(["xdg-open", "tg://resolve?domain=Drive502_Bot"])

    def start_service(self, _):
        run_cmd(f"systemctl --user start {SERVICE_NAME}")
        notify("502Drive", "Đã khởi động dịch vụ bot.", ICON_ACTIVE)
        self.update_status()

    def stop_service(self, _):
        run_cmd(f"systemctl --user stop {SERVICE_NAME}")
        notify("502Drive", "Đã tạm dừng dịch vụ bot.", ICON_INACTIVE)
        self.update_status()

    def restart_service(self, _):
        run_cmd(f"systemctl --user restart {SERVICE_NAME}")
        notify("502Drive", "Đã khởi động lại dịch vụ bot.", ICON_ACTIVE)
        self.update_status()

    def login_google(self, _):
        open_in_terminal("502Drive - Đăng nhập Google", "502drive auth login")

    def revoke_google(self, _):
        res = subprocess.run([
            "zenity", "--question",
            "--title=502Drive - Xác nhận",
            "--text=Bạn có chắc chắn muốn huỷ kết nối tài khoản Google hiện tại khỏi 502Drive?",
            "--width=360"
        ])
        if res.returncode == 0:
            ok, out, err = run_cmd("502drive auth revoke")
            run_cmd(f"systemctl --user restart {SERVICE_NAME}")
            notify("502Drive", "Đã huỷ kết nối tài khoản Google.", ICON_INACTIVE)
            self.build_menu()

    def open_reports(self, _):
        REPORTS_DIR.mkdir(parents=True, exist_ok=True)
        subprocess.Popen(["xdg-open", str(REPORTS_DIR)])

    def run_backup(self, _):
        backup_dest = Path.home() / "502drive-backup"
        ok, out, err = run_cmd(f"502drive backup {backup_dest}")
        if ok:
            notify("502Drive Backup", f"Đã sao lưu thành công vào: {backup_dest}")
        else:
            notify("502Drive Backup", f"Lỗi sao lưu: {err or out}")

    def run_doctor(self, _):
        ok, out, err = run_cmd("502drive doctor")
        msg = out if ok else (err or out or "Lỗi khi kiểm tra doctor")
        subprocess.Popen([
            "zenity", "--info",
            "--title=502Drive Doctor Report",
            "--text=" + msg,
            "--width=500", "--height=360"
        ])

    def view_logs(self, _):
        open_in_terminal("502Drive Live Logs", f"journalctl --user -u {SERVICE_NAME} -f")

    def edit_config(self, _):
        if CONFIG_PATH.exists():
            subprocess.Popen(["xdg-open", str(CONFIG_PATH)])
        else:
            notify("502Drive", f"Không tìm thấy file {CONFIG_PATH}")

    def set_autostart(self, widget, enable):
        if self.building_menu or not widget.get_active():
            return
        AUTOSTART_DESKTOP.parent.mkdir(parents=True, exist_ok=True)
        if enable:
            run_cmd(f"systemctl --user enable {SERVICE_NAME}")
            if INSTALLED_DESKTOP.exists():
                subprocess.run(["cp", str(INSTALLED_DESKTOP), str(AUTOSTART_DESKTOP)])
            notify("502Drive", "Đã bật khởi động cùng hệ thống.")
        else:
            run_cmd(f"systemctl --user disable {SERVICE_NAME}")
            if AUTOSTART_DESKTOP.exists():
                AUTOSTART_DESKTOP.unlink(missing_ok=True)
            notify("502Drive", "Đã tắt khởi động cùng hệ thống.")
        GLib.idle_add(self.build_menu)

    def set_lang(self, widget, lang):
        if self.building_menu or not widget.get_active():
            return
        if CONFIG_PATH.exists():
            try:
                content = CONFIG_PATH.read_text(encoding="utf-8")
                lines = []
                found = False
                for line in content.splitlines():
                    if line.strip().startswith("language"):
                        lines.append(f'language = "{lang}"')
                        found = True
                    else:
                        lines.append(line)
                if not found:
                    lines.append(f'language = "{lang}"')
                CONFIG_PATH.write_text("\n".join(lines) + "\n", encoding="utf-8")
                run_cmd(f"systemctl --user restart {SERVICE_NAME}")
                notify("502Drive", f"Đã chuyển ngôn ngữ sang: {'Tiếng Việt' if lang == 'vi' else 'English'}")
            except Exception as e:
                notify("502Drive", f"Lỗi đổi ngôn ngữ: {e}")
        GLib.idle_add(self.build_menu)

    def quit_app(self, _):
        Gtk.main_quit()


def main():
    signal.signal(signal.SIGINT, signal.SIG_DFL)
    _app = DriveTray()
    Gtk.main()


if __name__ == "__main__":
    main()
