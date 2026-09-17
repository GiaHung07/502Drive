#!/usr/bin/env python3
"""
502Drive Tray Controller
- Compact, silky-smooth System Tray Controller adhering to 502Print standards (Zero Emojis, No Lag).
- Launches and manages official 502Drive v2 Desktop GUI (Tauri).
- Fully localized in Vietnamese and English (Dynamic i18n).
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
APP_VERSION = "v0.2.0"
ICON_ACTIVE = "502drive-symbolic"
ICON_INACTIVE = "502drive-inactive-symbolic"
ICON_THEME_PATH = str(Path.home() / ".local/share/icons")
CONFIG_PATH = Path.home() / ".config" / "gdclone-bot" / "config.toml"
DB_PATH = Path.home() / ".local" / "share" / "gdclone-bot" / "state.db"
REPORTS_DIR = Path.home() / ".local" / "share" / "gdclone-bot" / "reports"
AUTOSTART_DESKTOP = Path.home() / ".config" / "autostart" / "502drive.desktop"
INSTALLED_DESKTOP = Path.home() / ".local" / "share" / "applications" / "502drive.desktop"
PID_FILE = Path(f"/tmp/502drive-tray-{os.getuid()}.pid")

# ── Localization Dictionary ──────────────────────────────────────────────────
I18N = {
    "vi": {
        "open_dashboard": "Mở Bảng điều khiển...",
        "open_telegram": "Mở Telegram...",
        "google_account": "Tài khoản Google",
        "acc_connected": "{email}",
        "acc_disconnected": "Chưa kết nối",
        "login_new": "Đăng nhập mới...",
        "disconnect_google": "Ngắt kết nối...",
        "bot_service": "Dịch vụ 502Drive",
        "status_running": "Đang hoạt động",
        "status_stopped": "Đang dừng",
        "start_service": "Khởi động",
        "stop_service": "Tạm dừng",
        "restart_service": "Khởi động lại",
        "admin_data": "Quản trị & Dữ liệu",
        "doctor": "Kiểm tra (Doctor)...",
        "open_reports": "Thư mục Reports...",
        "backup": "Sao lưu (Backup)...",
        "live_logs": "Xem Live Logs...",
        "config_file": "Cấu hình (config)...",
        "autostart": "Khởi động cùng hệ thống",
        "enable_autostart": "Đang bật",
        "disable_autostart": "Đang tắt",
        "language": "Ngôn ngữ / Language",
        "lang_vi": "Tiếng Việt",
        "lang_en": "English",
        "quit": "Thoát 502Drive",
        "service_started": "Đã khởi động dịch vụ 502Drive.",
        "service_stopped": "Đã tạm dừng dịch vụ 502Drive.",
        "service_restarted": "Đã khởi động lại dịch vụ 502Drive.",
        "account_revoked": "Đã huỷ kết nối tài khoản Google.",
        "backup_success": "Đã sao lưu thành công vào: {dest}",
        "backup_failed": "Lỗi sao lưu: {err}",
        "autostart_on": "Đã bật khởi động cùng hệ thống.",
        "autostart_off": "Đã tắt khởi động cùng hệ thống.",
        "lang_switched": "Đã chuyển ngôn ngữ sang: {lang}",
        "confirm_revoke_title": "502Drive - Xác nhận",
        "confirm_revoke_text": "Bạn có chắc chắn muốn huỷ kết nối tài khoản Google khỏi 502Drive?",
        "press_enter_to_close": "Nhấn Enter để đóng...",
    },
    "en": {
        "open_dashboard": "Open Dashboard...",
        "open_telegram": "Open Telegram...",
        "google_account": "Google Account",
        "acc_connected": "{email}",
        "acc_disconnected": "Not Connected",
        "login_new": "Log In New...",
        "disconnect_google": "Disconnect...",
        "bot_service": "502Drive Service",
        "status_running": "Running",
        "status_stopped": "Stopped",
        "start_service": "Start",
        "stop_service": "Stop",
        "restart_service": "Restart",
        "admin_data": "Admin & Data",
        "doctor": "Doctor Check...",
        "open_reports": "Reports Folder...",
        "backup": "Backup Data...",
        "live_logs": "Live Logs...",
        "config_file": "Configuration...",
        "autostart": "System Autostart",
        "enable_autostart": "Enabled",
        "disable_autostart": "Disabled",
        "language": "Language / Ngôn ngữ",
        "lang_vi": "Tiếng Việt",
        "lang_en": "English",
        "quit": "Exit 502Drive",
        "service_started": "502Drive service started.",
        "service_stopped": "502Drive service stopped.",
        "service_restarted": "502Drive service restarted.",
        "account_revoked": "Google Drive disconnected.",
        "backup_success": "Backup created at: {dest}",
        "backup_failed": "Backup failed: {err}",
        "autostart_on": "Autostart enabled.",
        "autostart_off": "Autostart disabled.",
        "lang_switched": "Switched language to: {lang}",
        "confirm_revoke_title": "502Drive - Confirmation",
        "confirm_revoke_text": "Disconnect Google Drive account from 502Drive?",
        "press_enter_to_close": "Press Enter to close...",
    }
}

def t(key, lang="vi", **kwargs):
    bundle = I18N.get(lang, I18N["vi"])
    template = bundle.get(key, I18N["vi"].get(key, key))
    if kwargs:
        try:
            return template.format(**kwargs)
        except Exception:
            return template
    return template


def truncate_middle(text, max_len=22):
    if not text or len(text) <= max_len:
        return text
    part = (max_len - 3) // 2
    return f"{text[:part]}...{text[-part:]}"


def run_cmd(cmd):
    try:
        res = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=5)
        return res.returncode == 0, res.stdout.strip(), res.stderr.strip()
    except Exception as e:
        return False, "", str(e)


def notify(title, message, icon="502drive-symbolic"):
    subprocess.Popen(["notify-send", "-a", APP_NAME, "-i", icon, title, message])


def get_terminal_cmd():
    for term in ["ptyxis", "gnome-terminal", "kgx", "xterm"]:
        if subprocess.run(f"which {term}", shell=True, capture_output=True).returncode == 0:
            return term
    return None


def open_in_terminal(title, command, lang="vi"):
    term = get_terminal_cmd()
    prompt = t("press_enter_to_close", lang)
    if term == "ptyxis":
        subprocess.Popen(["ptyxis", "--title", title, "--", "bash", "-c", f"{command}; echo; read -p '{prompt}'"])
    elif term == "gnome-terminal":
        subprocess.Popen(["gnome-terminal", "--title", title, "--", "bash", "-c", f"{command}; echo; read -p '{prompt}'"])
    elif term:
        subprocess.Popen([term, "-e", f"bash -c \"{command}; echo; read -p '{prompt}'\""])
    else:
        notify(title, f"Run: {command}")


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


# ── Silky-Smooth Compact Tray Controller ───────────────────────────────────────
class DriveTray:
    def __init__(self):
        self.last_status = None
        self.last_google_email = None
        self.lang = self.get_config_language()

        icon = ICON_ACTIVE if self.is_service_active() else ICON_INACTIVE
        self.indicator = AyatanaAppIndicator3.Indicator.new(
            APP_ID,
            icon,
            AyatanaAppIndicator3.IndicatorCategory.APPLICATION_STATUS,
        )
        try:
            self.indicator.set_icon_theme_path(ICON_THEME_PATH)
        except Exception:
            pass
        self.indicator.set_status(AyatanaAppIndicator3.IndicatorStatus.ACTIVE)

        self.menu = Gtk.Menu()
        self.indicator.set_menu(self.menu)

        self.build_menu()

        # Non-blocking status refresher (every 4 seconds, minimal footprint)
        GLib.timeout_add_seconds(4, self.update_status)

    def is_service_active(self):
        ok, out, _ = run_cmd(f"systemctl --user is-active {SERVICE_NAME}")
        return ok and out == "active"

    def get_google_account_email(self):
        if not DB_PATH.exists():
            return None
        try:
            conn = sqlite3.connect(f"file:{DB_PATH}?mode=ro", uri=True)
            cursor = conn.cursor()
            cursor.execute("SELECT email, label, status FROM google_accounts WHERE id = 'default'")
            row = cursor.fetchone()
            conn.close()
            if row and row[2] == "connected":
                return row[0] or row[1] or "Connected"
        except Exception:
            pass
        return None

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

    def set_autostart_state(self, enable):
        AUTOSTART_DESKTOP.parent.mkdir(parents=True, exist_ok=True)
        if enable:
            run_cmd(f"systemctl --user enable {SERVICE_NAME}")
            if INSTALLED_DESKTOP.exists():
                subprocess.run(["cp", str(INSTALLED_DESKTOP), str(AUTOSTART_DESKTOP)])
            notify("502Drive", t("autostart_on", self.lang))
        else:
            run_cmd(f"systemctl --user disable {SERVICE_NAME}")
            if AUTOSTART_DESKTOP.exists():
                AUTOSTART_DESKTOP.unlink(missing_ok=True)
            notify("502Drive", t("autostart_off", self.lang))

    def show_dashboard(self, _=None):
        gui_bin = Path.home() / ".local/bin/502drive-gui"
        cmd = str(gui_bin) if gui_bin.exists() else "502drive-gui"
        try:
            subprocess.Popen([cmd], start_new_session=True)
        except Exception as e:
            notify("502Drive", f"Không thể mở giao diện 502Drive: {e}")

    def build_menu(self):
        # Clear once
        for child in self.menu.get_children():
            self.menu.remove(child)

        lang = self.lang
        active = self.is_service_active()
        google_email = self.get_google_account_email()
        self.last_status = active
        self.last_google_email = google_email

        # 1. Primary Action: Mở Bảng điều khiển (Launches 502drive-gui v2 Desktop App)
        dash_item = make_item(
            t("open_dashboard", lang),
            icon_name="document-open-symbolic",
            callback=self.show_dashboard,
        )
        self.menu.append(dash_item)

        # 2. Telegram Link
        tg_item = make_item(
            t("open_telegram", lang),
            icon_name="send-to-symbolic",
            callback=self.open_telegram,
        )
        self.menu.append(tg_item)

        self.menu.append(Gtk.SeparatorMenuItem())

        # 3. Google Account Submenu
        google_item, google_sub = make_submenu(t("google_account", lang), icon_name="avatar-default-symbolic")
        if google_email:
            acc_label = truncate_middle(google_email, 22)
            icon_status = "emblem-default-symbolic"
        else:
            acc_label = t("acc_disconnected", lang)
            icon_status = "dialog-warning-symbolic"

        self.acc_status_item = make_item(acc_label, icon_name=icon_status, sensitive=False)
        google_sub.append(self.acc_status_item)
        google_sub.append(Gtk.SeparatorMenuItem())

        login_item = make_item(
            t("login_new", lang),
            icon_name="document-open-symbolic",
            callback=self.login_google,
        )
        google_sub.append(login_item)

        self.revoke_item = make_item(
            t("disconnect_google", lang),
            icon_name="edit-delete-symbolic",
            callback=self.revoke_google,
            sensitive=bool(google_email),
        )
        google_sub.append(self.revoke_item)
        self.menu.append(google_item)

        # 4. Service Submenu
        service_item, service_sub = make_submenu(t("bot_service", lang), icon_name="system-run-symbolic")
        status_text = t("status_running", lang) if active else t("status_stopped", lang)
        status_icon = "emblem-default-symbolic" if active else "action-unavailable-symbolic"
        self.service_status_item = make_item(status_text, icon_name=status_icon, sensitive=False)
        service_sub.append(self.service_status_item)
        service_sub.append(Gtk.SeparatorMenuItem())

        start_item = make_item(t("start_service", lang), icon_name="media-playback-start-symbolic", callback=self.start_service)
        stop_item = make_item(t("stop_service", lang), icon_name="media-playback-pause-symbolic", callback=self.stop_service)
        restart_item = make_item(t("restart_service", lang), icon_name="view-refresh-symbolic", callback=self.restart_service)
        service_sub.append(start_item)
        service_sub.append(stop_item)
        service_sub.append(restart_item)
        self.menu.append(service_item)

        # 5. Admin & Data Submenu
        admin_item, admin_sub = make_submenu(t("admin_data", lang), icon_name="preferences-system-symbolic")
        doctor_item = make_item(t("doctor", lang), icon_name="utilities-system-monitor-symbolic", callback=self.run_doctor)
        logs_item = make_item(t("live_logs", lang), icon_name="utilities-terminal-symbolic", callback=self.view_logs)
        backup_item = make_item(t("backup", lang), icon_name="drive-harddisk-symbolic", callback=self.run_backup)
        reports_item = make_item(t("open_reports", lang), icon_name="folder-open-symbolic", callback=self.open_reports)
        config_item = make_item(t("config_file", lang), icon_name="text-x-generic-symbolic", callback=self.edit_config)

        admin_sub.append(doctor_item)
        admin_sub.append(logs_item)
        admin_sub.append(backup_item)
        admin_sub.append(reports_item)
        admin_sub.append(config_item)
        self.menu.append(admin_item)

        self.menu.append(Gtk.SeparatorMenuItem())

        # 6. Autostart Submenu
        autostart_item, autostart_sub = make_submenu(t("autostart", lang), icon_name="system-run-symbolic")
        autostart_on = self.is_autostart_enabled()
        self.as_on_item = Gtk.RadioMenuItem.new_with_label(None, t("enable_autostart", lang))
        self.as_off_item = Gtk.RadioMenuItem.new_with_label_from_widget(self.as_on_item, t("disable_autostart", lang))
        self.as_on_item.set_active(autostart_on)
        self.as_off_item.set_active(not autostart_on)
        self.as_on_item.connect("toggled", lambda w: self.set_autostart(w, True))
        self.as_off_item.connect("toggled", lambda w: self.set_autostart(w, False))
        autostart_sub.append(self.as_on_item)
        autostart_sub.append(self.as_off_item)
        self.menu.append(autostart_item)

        # 7. Language Submenu
        lang_item, lang_sub = make_submenu(t("language", lang), icon_name="preferences-desktop-locale-symbolic")
        self.vi_item = Gtk.RadioMenuItem.new_with_label(None, t("lang_vi", lang))
        self.en_item = Gtk.RadioMenuItem.new_with_label_from_widget(self.vi_item, t("lang_en", lang))
        self.vi_item.set_active(lang == "vi")
        self.en_item.set_active(lang == "en")
        self.vi_item.connect("toggled", lambda w: self.switch_language(w, "vi"))
        self.en_item.connect("toggled", lambda w: self.switch_language(w, "en"))
        lang_sub.append(self.vi_item)
        lang_sub.append(self.en_item)
        self.menu.append(lang_item)

        self.menu.append(Gtk.SeparatorMenuItem())

        # 8. Quit
        quit_item = make_item(
            t("quit", lang),
            icon_name="application-exit-symbolic",
            callback=self.quit_app,
        )
        self.menu.append(quit_item)

        self.menu.show_all()

    def update_status(self):
        active = self.is_service_active()
        google_email = self.get_google_account_email()
        lang = self.get_config_language()

        if lang != self.lang:
            self.lang = lang
            self.build_menu()
            return True

        if active != self.last_status:
            self.last_status = active
            if active:
                self.indicator.set_icon_full(ICON_ACTIVE, "Active")
                self.service_status_item.set_label(t("status_running", self.lang))
            else:
                self.indicator.set_icon_full(ICON_INACTIVE, "Inactive")
                self.service_status_item.set_label(t("status_stopped", self.lang))

        if google_email != self.last_google_email:
            self.last_google_email = google_email
            if google_email:
                self.acc_status_item.set_label(truncate_middle(google_email, 22))
                self.revoke_item.set_sensitive(True)
            else:
                self.acc_status_item.set_label(t("acc_disconnected", self.lang))
                self.revoke_item.set_sensitive(False)

        return True

    def open_telegram(self, _):
        subprocess.Popen(["xdg-open", "tg://resolve?domain=Drive502_Bot"])

    def start_service(self, _):
        run_cmd(f"systemctl --user start {SERVICE_NAME}")
        notify("502Drive", t("service_started", self.lang), ICON_ACTIVE)
        self.update_status()

    def stop_service(self, _):
        run_cmd(f"systemctl --user stop {SERVICE_NAME}")
        notify("502Drive", t("service_stopped", self.lang), ICON_INACTIVE)
        self.update_status()

    def restart_service(self, _):
        run_cmd(f"systemctl --user restart {SERVICE_NAME}")
        notify("502Drive", t("service_restarted", self.lang), ICON_ACTIVE)
        self.update_status()

    def login_google(self, _):
        title = "502Drive - " + ("Google Login" if self.lang == "en" else "Đăng nhập Google")
        open_in_terminal(title, "502drive auth login", self.lang)

    def revoke_google(self, _):
        title = t("confirm_revoke_title", self.lang)
        prompt = t("confirm_revoke_text", self.lang)
        res = subprocess.run([
            "zenity", "--question",
            f"--title={title}",
            f"--text={prompt}",
            "--width=360"
        ])
        if res.returncode == 0:
            ok, out, err = run_cmd("502drive auth revoke")
            run_cmd(f"systemctl --user restart {SERVICE_NAME}")
            notify("502Drive", t("account_revoked", self.lang), ICON_INACTIVE)
            self.update_status()

    def open_reports(self, _):
        REPORTS_DIR.mkdir(parents=True, exist_ok=True)
        subprocess.Popen(["xdg-open", str(REPORTS_DIR)])

    def run_backup(self, _):
        backup_dest = Path.home() / "502drive-backup"
        ok, out, err = run_cmd(f"502drive backup {backup_dest}")
        if ok:
            notify("502Drive Backup", t("backup_success", self.lang, dest=backup_dest))
        else:
            notify("502Drive Backup", t("backup_failed", self.lang, err=(err or out)))

    def run_doctor(self, _):
        open_in_terminal("502Drive Doctor", "502drive doctor", self.lang)

    def view_logs(self, _):
        open_in_terminal("502Drive Live Logs", f"journalctl --user -u {SERVICE_NAME} -f", self.lang)

    def edit_config(self, _):
        if CONFIG_PATH.exists():
            subprocess.Popen(["xdg-open", str(CONFIG_PATH)])
        else:
            notify("502Drive", f"Not found: {CONFIG_PATH}")

    def set_autostart(self, widget, enable):
        if not widget.get_active():
            return
        self.set_autostart_state(enable)

    def switch_language(self, widget, target_lang):
        if not widget.get_active() or target_lang == self.lang:
            return
        if not CONFIG_PATH.exists():
            return
        try:
            with open(CONFIG_PATH, "r", encoding="utf-8") as f:
                lines = f.readlines()
            new_lines = []
            for line in lines:
                if line.strip().startswith("language"):
                    new_lines.append(f'language = "{target_lang}"\n')
                else:
                    new_lines.append(line)
            with open(CONFIG_PATH, "w", encoding="utf-8") as f:
                f.writelines(new_lines)

            run_cmd(f"systemctl --user restart {SERVICE_NAME}")
            self.lang = target_lang
            self.build_menu()
            notify("502Drive", t("lang_switched", self.lang, lang=target_lang))
        except Exception as e:
            notify("502Drive", f"Error switching language: {e}")

    def quit_app(self, _):
        PID_FILE.unlink(missing_ok=True)
        Gtk.main_quit()


def main():
    signal.signal(signal.SIGINT, signal.SIG_DFL)

    # If --gui or open requested, launch 502drive-gui desktop app directly
    if len(sys.argv) > 1 and any(arg in sys.argv[1:] for arg in ["--window", "--gui", "gui", "open"]):
        gui_bin = Path.home() / ".local/bin/502drive-gui"
        cmd = str(gui_bin) if gui_bin.exists() else "502drive-gui"
        try:
            subprocess.Popen([cmd], start_new_session=True)
        except Exception as e:
            notify("502Drive", f"Không thể mở 502Drive: {e}")
        sys.exit(0)

    tray = DriveTray()
    Gtk.main()


if __name__ == "__main__":
    main()
