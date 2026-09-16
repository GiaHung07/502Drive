#!/usr/bin/env python3
"""
502Drive Desktop Application & Tray Controller
- Compact, silky-smooth Tray Controller adhering to 502Print standards (Zero Emojis, No Lag).
- Native Desktop App Window (LANDrop style) with Home, Jobs/Sync, Settings, and Dev CLI tabs.
- Fully localized in Vietnamese and English (Dynamic i18n).
"""

import os
import signal
import sqlite3
import subprocess
import sys
import threading
import tomllib
import urllib.request
import json
from pathlib import Path

import gi
gi.require_version("Gtk", "3.0")
gi.require_version("AyatanaAppIndicator3", "0.1")
from gi.repository import AyatanaAppIndicator3, GLib, Gtk, Gdk

SERVICE_NAME = "gdclone-bot"
APP_NAME = "502Drive"
APP_ID = "502drive"
APP_VERSION = "v0.1.0"
ICON_ACTIVE = "502drive-symbolic"  # icon name (hicolor theme lookup - scales correctly at all sizes)
ICON_INACTIVE = "502drive-inactive-symbolic"  # icon name
ICON_APP = str(Path.home() / ".local/share/icons/hicolor/scalable/apps/502drive.svg")
# Fallback paths for indicator icon theme
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
        "open_telegram": "Mở Telegram Bot...",
        "google_account": "Tài khoản Google",
        "acc_connected": "{email}",
        "acc_disconnected": "Chưa kết nối",
        "login_new": "Đăng nhập mới...",
        "disconnect_google": "Ngắt kết nối...",
        "bot_service": "Dịch vụ Bot",
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
        "service_started": "Đã khởi động dịch vụ bot.",
        "service_stopped": "Đã tạm dừng dịch vụ bot.",
        "service_restarted": "Đã khởi động lại dịch vụ bot.",
        "account_revoked": "Đã huỷ kết nối tài khoản Google.",
        "backup_success": "Đã sao lưu thành công vào: {dest}",
        "backup_failed": "Lỗi sao lưu: {err}",
        "autostart_on": "Đã bật khởi động cùng hệ thống.",
        "autostart_off": "Đã tắt khởi động cùng hệ thống.",
        "lang_switched": "Đã chuyển ngôn ngữ sang: {lang}",
        "confirm_revoke_title": "502Drive - Xác nhận",
        "confirm_revoke_text": "Bạn có chắc chắn muốn huỷ kết nối tài khoản Google khỏi 502Drive?",
        "press_enter_to_close": "Nhấn Enter để đóng...",
        # Window UI
        "win_title": "502Drive",
        "nav_home": "Trang chính",
        "nav_jobs": "Tiến độ & Đồng bộ",
        "nav_settings": "Cài đặt",
        "nav_dev": "Công cụ Dev",
        "device_name": "Tên thiết bị (Device Name)",
        "device_desc": "Tên định danh thiết bị hiển thị trong báo cáo và hệ thống.",
        "dest_path": "Thư mục đích (Destination Folder)",
        "dest_desc": "Đường dẫn hoặc Folder ID Google Drive lưu trữ mặc định.",
        "launch_startup": "Tự khởi động cùng hệ thống (Launch at Startup)",
        "launch_desc": "Tự động kích hoạt bot và khay điều khiển ngầm khi bật máy tính.",
        "auto_confirm": "Tự động xác nhận sao chép (Auto-confirm Clone)",
        "auto_confirm_desc": "Bắt đầu sao chép ngay khi gửi link mà không cần bấm xác nhận.",
        "engine_concurrency": "Tiến trình đồng thời (Engine Concurrency)",
        "engine_desc": "Số luồng ghi tệp đồng thời để tối ưu hiệu năng sao chép.",
        "app_language": "Ngôn ngữ giao diện (Language)",
        "lang_desc": "Ngôn ngữ cho Bot Telegram, khay hệ thống và ứng dụng.",
        "software_update": "Cập nhật phần mềm (Software Updates)",
        "update_desc": "Kiểm tra và tự động cập nhật phiên bản mới nhất từ GitHub.",
        "check_update": "Kiểm tra bản mới...",
        "save_settings": "Lưu cài đặt",
        "settings_saved": "Đã lưu cài đặt và áp dụng thành công!",
        "current_ver": "Phiên bản hiện tại: {ver}",
        "checking": "Đang kiểm tra...",
        "up_to_date": "Bạn đang dùng phiên bản mới nhất ({ver}).",
        "new_version_found": "Đã có bản mới {ver}! Hãy tải từ GitHub.",
    },
    "en": {
        "open_dashboard": "Open Dashboard...",
        "open_telegram": "Open Telegram Bot...",
        "google_account": "Google Account",
        "acc_connected": "{email}",
        "acc_disconnected": "Not Connected",
        "login_new": "Log In New...",
        "disconnect_google": "Disconnect...",
        "bot_service": "Bot Service",
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
        "service_started": "Bot service started.",
        "service_stopped": "Bot service stopped.",
        "service_restarted": "Bot service restarted.",
        "account_revoked": "Google Drive disconnected.",
        "backup_success": "Backup created at: {dest}",
        "backup_failed": "Backup failed: {err}",
        "autostart_on": "Autostart enabled.",
        "autostart_off": "Autostart disabled.",
        "lang_switched": "Switched language to: {lang}",
        "confirm_revoke_title": "502Drive - Confirmation",
        "confirm_revoke_text": "Disconnect Google Drive account from 502Drive?",
        "press_enter_to_close": "Press Enter to close...",
        # Window UI
        "win_title": "502Drive",
        "nav_home": "Home",
        "nav_jobs": "Jobs & Sync",
        "nav_settings": "Settings",
        "nav_dev": "Developer CLI",
        "device_name": "Device Name",
        "device_desc": "Identifier shown in reports and system logs.",
        "dest_path": "Destination Folder",
        "dest_desc": "Default Google Drive Folder URL or ID for downloads.",
        "launch_startup": "Launch at Startup",
        "launch_desc": "Run 502Drive silently in the background upon login.",
        "auto_confirm": "Auto-confirm Clone",
        "auto_confirm_desc": "Start cloning immediately without Telegram button confirmation.",
        "engine_concurrency": "Engine Concurrency",
        "engine_desc": "Maximum concurrent workers for copying files.",
        "app_language": "Language",
        "lang_desc": "Language for Telegram bot, tray, and application.",
        "software_update": "Software Updates",
        "update_desc": "Check and download updates from GitHub Releases.",
        "check_update": "Check for Updates...",
        "save_settings": "Save Settings",
        "settings_saved": "Settings saved and applied successfully!",
        "current_ver": "Current version: {ver}",
        "checking": "Checking...",
        "up_to_date": "You are on the latest version ({ver}).",
        "new_version_found": "New version {ver} is available on GitHub!",
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


# ── Modern Adwaita / LANDrop Dark CSS ──────────────────────────────────────────
APP_CSS = """
window {
    background-color: #1a1d24;
    color: #e2e8f0;
}
.sidebar {
    background-color: #12141a;
    border-right: 1px solid #262b36;
    padding: 16px 12px;
}
.app-title {
    font-size: 16px;
    font-weight: bold;
    color: #ffffff;
}
.app-version {
    font-size: 11px;
    color: #718096;
}
.nav-button {
    padding: 10px 14px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    color: #cbd5e0;
    border: none;
    background: transparent;
}
.nav-button:hover {
    background-color: #20242e;
    color: #ffffff;
}
.nav-button:checked {
    background-color: #2b3240;
    color: #63b3ed;
    font-weight: bold;
}
.content-area {
    padding: 24px 32px;
}
.page-title {
    font-size: 20px;
    font-weight: bold;
    color: #ffffff;
    margin-bottom: 18px;
}
.card {
    background-color: #222631;
    border: 1px solid #2d3443;
    border-radius: 10px;
    padding: 18px 20px;
    margin-bottom: 14px;
}
.card-title {
    font-size: 14px;
    font-weight: bold;
    color: #ffffff;
}
.card-desc {
    font-size: 12px;
    color: #a0aec0;
}
.setting-row {
    margin-bottom: 18px;
}
.setting-title {
    font-size: 14px;
    font-weight: 600;
    color: #ffffff;
}
.setting-desc {
    font-size: 12px;
    color: #8c9ba5;
    margin-top: 2px;
}
.status-pill-ok {
    background-color: #1e3a29;
    color: #4ade80;
    border: 1px solid #2e7d32;
    border-radius: 12px;
    padding: 3px 10px;
    font-size: 11px;
    font-weight: bold;
}
.status-pill-warn {
    background-color: #3b2219;
    color: #f87171;
    border: 1px solid #991b1b;
    border-radius: 12px;
    padding: 3px 10px;
    font-size: 11px;
    font-weight: bold;
}
.console-box {
    background-color: #0f1117;
    border: 1px solid #262b36;
    border-radius: 8px;
    padding: 12px;
    font-family: monospace;
    font-size: 12px;
    color: #a0aec0;
}
"""


# ── Native App Window (LANDrop-Style Modern Desktop GUI) ───────────────────────
class DriveAppWindow(Gtk.Window):
    def __init__(self, tray_controller):
        super().__init__(title="502Drive")
        self.tray = tray_controller
        self.lang = tray_controller.lang
        self.set_default_size(840, 560)
        self.set_position(Gtk.WindowPosition.CENTER)
        self.set_icon_name("502drive")

        # Header bar
        header = Gtk.HeaderBar()
        header.set_show_close_button(True)
        header.set_title("502Drive")
        header.set_subtitle("Google Drive Sync & Telegram Bot")
        self.set_titlebar(header)

        # Root layout box
        root = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        self.add(root)

        # 1. Left Sidebar
        sidebar = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        sidebar.get_style_context().add_class("sidebar")
        sidebar.set_size_request(210, -1)
        root.pack_start(sidebar, False, False, 0)

        # Brand header
        top_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        top_box.set_margin_bottom(18)
        top_box.set_margin_start(4)
        if Path(ICON_APP).exists():
            try:
                from gi.repository import GdkPixbuf
                pb = GdkPixbuf.Pixbuf.new_from_file_at_scale(ICON_APP, 36, 36, True)
                icon_img = Gtk.Image.new_from_pixbuf(pb)
            except Exception:
                icon_img = Gtk.Image.new_from_file(ICON_APP)
            top_box.pack_start(icon_img, False, False, 0)

        info_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        t_label = Gtk.Label(xalign=0, label="502Drive")
        t_label.get_style_context().add_class("app-title")
        v_label = Gtk.Label(xalign=0, label=APP_VERSION)
        v_label.get_style_context().add_class("app-version")
        info_box.pack_start(t_label, False, False, 0)
        info_box.pack_start(v_label, False, False, 0)
        top_box.pack_start(info_box, True, True, 0)
        sidebar.pack_start(top_box, False, False, 0)

        # Navigation Buttons (Radio buttons without circle indicator)
        self.btn_home = Gtk.RadioButton.new_with_label(None, t("nav_home", self.lang))
        self.btn_home.set_mode(False)
        self.btn_home.get_style_context().add_class("nav-button")
        self.btn_home.connect("toggled", lambda b: self.on_nav_toggled("home", b))
        sidebar.pack_start(self.btn_home, False, False, 0)

        self.btn_jobs = Gtk.RadioButton.new_with_label_from_widget(self.btn_home, t("nav_jobs", self.lang))
        self.btn_jobs.set_mode(False)
        self.btn_jobs.get_style_context().add_class("nav-button")
        self.btn_jobs.connect("toggled", lambda b: self.on_nav_toggled("jobs", b))
        sidebar.pack_start(self.btn_jobs, False, False, 0)

        self.btn_settings = Gtk.RadioButton.new_with_label_from_widget(self.btn_home, t("nav_settings", self.lang))
        self.btn_settings.set_mode(False)
        self.btn_settings.get_style_context().add_class("nav-button")
        self.btn_settings.connect("toggled", lambda b: self.on_nav_toggled("settings", b))
        sidebar.pack_start(self.btn_settings, False, False, 0)

        self.btn_dev = Gtk.RadioButton.new_with_label_from_widget(self.btn_home, t("nav_dev", self.lang))
        self.btn_dev.set_mode(False)
        self.btn_dev.get_style_context().add_class("nav-button")
        self.btn_dev.connect("toggled", lambda b: self.on_nav_toggled("dev", b))
        sidebar.pack_start(self.btn_dev, False, False, 0)

        # Bottom Live Service Pill
        self.sidebar_status_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        self.sidebar_status_box.set_valign(Gtk.Align.END)
        self.sidebar_status_box.set_margin_bottom(8)
        self.lbl_sidebar_status = Gtk.Label(label="Bot: Đang chạy" if self.lang == "vi" else "Bot: Running")
        self.lbl_sidebar_status.get_style_context().add_class("status-pill-ok")
        self.sidebar_status_box.pack_start(self.lbl_sidebar_status, True, True, 0)
        sidebar.pack_end(self.sidebar_status_box, False, False, 0)

        # 2. Right Content Stack
        self.stack = Gtk.Stack()
        self.stack.set_transition_type(Gtk.StackTransitionType.CROSSFADE)
        self.stack.set_transition_duration(150)
        root.pack_start(self.stack, True, True, 0)

        # Build pages
        self.build_home_page()
        self.build_jobs_page()
        self.build_settings_page()
        self.build_dev_page()

        # Connect hide on close
        self.connect("delete-event", self.on_delete_event)

        # Ensure all widgets are completely visible
        self.show_all()

    def on_delete_event(self, widget, event):
        self.hide()
        return True

    def on_nav_toggled(self, name, btn):
        if btn.get_active():
            self.stack.set_visible_child_name(name)
            if name == "jobs":
                self.refresh_jobs_data()

    def build_home_page(self):
        page = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
        page.get_style_context().add_class("content-area")

        title = Gtk.Label(xalign=0, label="Trạng thái hệ thống" if self.lang == "vi" else "System Overview")
        title.get_style_context().add_class("page-title")
        page.pack_start(title, False, False, 0)

        # Card 1: Google Account
        card1 = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        card1.get_style_context().add_class("card")
        c1_left = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        c1_title = Gtk.Label(xalign=0, label="Tài khoản Google Drive" if self.lang == "vi" else "Google Drive Account")
        c1_title.get_style_context().add_class("card-title")
        self.c1_email = Gtk.Label(xalign=0)
        self.c1_email.get_style_context().add_class("card-desc")
        c1_left.pack_start(c1_title, False, False, 0)
        c1_left.pack_start(self.c1_email, False, False, 0)
        card1.pack_start(c1_left, True, True, 0)

        c1_actions = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        btn_login = Gtk.Button(label="Đăng nhập lại" if self.lang == "vi" else "Re-login")
        btn_login.connect("clicked", self.tray.login_google)
        c1_actions.pack_start(btn_login, False, False, 0)

        self.btn_revoke = Gtk.Button(label="Ngắt kết nối" if self.lang == "vi" else "Disconnect")
        self.btn_revoke.connect("clicked", self.tray.revoke_google)
        c1_actions.pack_start(self.btn_revoke, False, False, 0)
        card1.pack_end(c1_actions, False, False, 0)
        page.pack_start(card1, False, False, 0)

        # Card 2: Telegram Bot
        card2 = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        card2.get_style_context().add_class("card")
        c2_left = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        c2_title = Gtk.Label(xalign=0, label="Telegram Bot (@Drive502_Bot)")
        c2_title.get_style_context().add_class("card-title")
        self.c2_status = Gtk.Label(xalign=0)
        self.c2_status.get_style_context().add_class("card-desc")
        c2_left.pack_start(c2_title, False, False, 0)
        c2_left.pack_start(self.c2_status, False, False, 0)
        card2.pack_start(c2_left, True, True, 0)

        c2_actions = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        btn_tg = Gtk.Button(label="Mở Bot" if self.lang == "vi" else "Open Bot")
        btn_tg.connect("clicked", self.tray.open_telegram)
        c2_actions.pack_start(btn_tg, False, False, 0)

        btn_restart = Gtk.Button(label="Khởi động lại" if self.lang == "vi" else "Restart")
        btn_restart.connect("clicked", self.tray.restart_service)
        c2_actions.pack_start(btn_restart, False, False, 0)
        card2.pack_end(c2_actions, False, False, 0)
        page.pack_start(card2, False, False, 0)

        # Card 3: Destination
        card3 = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        card3.get_style_context().add_class("card")
        c3_left = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        c3_title = Gtk.Label(xalign=0, label="Thư mục Đích Google Drive" if self.lang == "vi" else "Target Google Drive Folder")
        c3_title.get_style_context().add_class("card-title")
        self.c3_desc = Gtk.Label(xalign=0)
        self.c3_desc.get_style_context().add_class("card-desc")
        c3_left.pack_start(c3_title, False, False, 0)
        c3_left.pack_start(self.c3_desc, False, False, 0)
        card3.pack_start(c3_left, True, True, 0)
        page.pack_start(card3, False, False, 0)

        self.stack.add_named(page, "home")

    def build_jobs_page(self):
        page = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
        page.get_style_context().add_class("content-area")

        top_bar = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        title = Gtk.Label(xalign=0, label="Tiến độ & Tác vụ Đồng bộ" if self.lang == "vi" else "Jobs & Realtime Sync")
        title.get_style_context().add_class("page-title")
        top_bar.pack_start(title, True, True, 0)

        btn_refresh = Gtk.Button(label="Làm mới" if self.lang == "vi" else "Refresh")
        btn_refresh.connect("clicked", lambda _: self.refresh_jobs_data())
        top_bar.pack_end(btn_refresh, False, False, 0)
        page.pack_start(top_bar, False, False, 0)

        # Scrolled Window for jobs list
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        page.pack_start(scrolled, True, True, 0)

        self.jobs_container = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        scrolled.add(self.jobs_container)

        self.stack.add_named(page, "jobs")

    def refresh_jobs_data(self):
        for child in self.jobs_container.get_children():
            self.jobs_container.remove(child)

        jobs = []
        watches = []
        if DB_PATH.exists():
            try:
                conn = sqlite3.connect(f"file:{DB_PATH}?mode=ro", uri=True)
                c = conn.cursor()
                c.execute("SELECT id, status, total_discovered, completed_items, failed_items FROM jobs ORDER BY updated_at_ms DESC LIMIT 8")
                jobs = c.fetchall()
                c.execute("SELECT id, source_root_id, destination_root_id, status FROM watch_subscriptions LIMIT 8")
                watches = c.fetchall()
                conn.close()
            except Exception:
                pass

        if not jobs and not watches:
            empty_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
            empty_box.set_valign(Gtk.Align.CENTER)
            empty_box.set_margin_top(40)
            lbl = Gtk.Label(label="Hiện không có tác vụ sao chép hoặc đồng bộ nào." if self.lang == "vi" else "No active jobs or folder watches.")
            lbl.get_style_context().add_class("card-desc")
            sublbl = Gtk.Label(label="Gửi liên kết Google Drive cho @Drive502_Bot để bắt đầu." if self.lang == "vi" else "Send a Google Drive link to @Drive502_Bot to start.")
            sublbl.get_style_context().add_class("app-version")
            empty_box.pack_start(lbl, False, False, 0)
            empty_box.pack_start(sublbl, False, False, 0)
            self.jobs_container.pack_start(empty_box, True, True, 0)
        else:
            for j in jobs:
                jid, status, total, done, fail = j
                card = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
                card.get_style_context().add_class("card")

                h = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
                t_lbl = Gtk.Label(xalign=0, label=f"Job #{jid[:8]} ({status})")
                t_lbl.get_style_context().add_class("card-title")
                cnt_lbl = Gtk.Label(xalign=1, label=f"{done}/{total} tệp" if self.lang == "vi" else f"{done}/{total} items")
                cnt_lbl.get_style_context().add_class("card-desc")
                h.pack_start(t_lbl, True, True, 0)
                h.pack_end(cnt_lbl, False, False, 0)
                card.pack_start(h, False, False, 0)

                pb = Gtk.ProgressBar()
                fraction = (done / max(total, 1)) if total > 0 else 0.0
                pb.set_fraction(fraction)
                card.pack_start(pb, False, False, 0)
                self.jobs_container.pack_start(card, False, False, 0)

            for w in watches:
                wid, src, dst, wstatus = w
                card = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
                card.get_style_context().add_class("card")
                left = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
                w_title = Gtk.Label(xalign=0, label=f"Sync Watch #{wid[:8]}")
                w_title.get_style_context().add_class("card-title")
                w_desc = Gtk.Label(xalign=0, label=f"Source: {src[:12]}... -> Status: {wstatus}")
                w_desc.get_style_context().add_class("card-desc")
                left.pack_start(w_title, False, False, 0)
                left.pack_start(w_desc, False, False, 0)
                card.pack_start(left, True, True, 0)
                self.jobs_container.pack_start(card, False, False, 0)

        self.jobs_container.show_all()

    def build_settings_page(self):
        # Exact clean layout matching LANDrop image!
        page = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
        page.get_style_context().add_class("content-area")

        title = Gtk.Label(xalign=0, label="Settings" if self.lang == "en" else "Cài đặt (Settings)")
        title.get_style_context().add_class("page-title")
        page.pack_start(title, False, False, 0)

        # Load config values
        cfg = self.load_config_data()

        # 1. Device Name
        row1 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        row1.get_style_context().add_class("setting-row")
        t1 = Gtk.Label(xalign=0, label=t("device_name", self.lang))
        t1.get_style_context().add_class("setting-title")
        self.entry_device = Gtk.Entry()
        self.entry_device.set_text(os.uname().nodename if hasattr(os, "uname") else "user-pgh")
        self.entry_device.set_sensitive(False)
        d1 = Gtk.Label(xalign=0, label=t("device_desc", self.lang))
        d1.get_style_context().add_class("setting-desc")
        row1.pack_start(t1, False, False, 0)
        row1.pack_start(self.entry_device, False, False, 0)
        row1.pack_start(d1, False, False, 0)
        page.pack_start(row1, False, False, 0)

        # 2. Auto-confirm Clone Toggle (LANDrop Style)
        row2 = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        row2.get_style_context().add_class("setting-row")
        r2_left = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
        t2 = Gtk.Label(xalign=0, label=t("auto_confirm", self.lang))
        t2.get_style_context().add_class("setting-title")
        d2 = Gtk.Label(xalign=0, label=t("auto_confirm_desc", self.lang))
        d2.get_style_context().add_class("setting-desc")
        r2_left.pack_start(t2, False, False, 0)
        r2_left.pack_start(d2, False, False, 0)
        row2.pack_start(r2_left, True, True, 0)

        self.sw_auto_confirm = Gtk.Switch()
        self.sw_auto_confirm.set_valign(Gtk.Align.CENTER)
        self.sw_auto_confirm.set_active(cfg.get("destination", {}).get("auto_confirm_clone", False))
        row2.pack_end(self.sw_auto_confirm, False, False, 0)
        page.pack_start(row2, False, False, 0)

        # 3. Launch at Startup (LANDrop Style)
        row3 = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        row3.get_style_context().add_class("setting-row")
        r3_left = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
        t3 = Gtk.Label(xalign=0, label=t("launch_startup", self.lang))
        t3.get_style_context().add_class("setting-title")
        d3 = Gtk.Label(xalign=0, label=t("launch_desc", self.lang))
        d3.get_style_context().add_class("setting-desc")
        r3_left.pack_start(t3, False, False, 0)
        r3_left.pack_start(d3, False, False, 0)
        row3.pack_start(r3_left, True, True, 0)

        self.sw_startup = Gtk.Switch()
        self.sw_startup.set_valign(Gtk.Align.CENTER)
        self.sw_startup.set_active(self.tray.is_autostart_enabled())
        self.sw_startup.connect("notify::active", self.on_startup_toggled)
        row3.pack_end(self.sw_startup, False, False, 0)
        page.pack_start(row3, False, False, 0)

        # 4. Check for Updates
        row4 = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        row4.get_style_context().add_class("setting-row")
        r4_left = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
        t4 = Gtk.Label(xalign=0, label=t("software_update", self.lang))
        t4.get_style_context().add_class("setting-title")
        self.lbl_update_status = Gtk.Label(xalign=0, label=t("current_ver", self.lang, ver=APP_VERSION))
        self.lbl_update_status.get_style_context().add_class("setting-desc")
        r4_left.pack_start(t4, False, False, 0)
        r4_left.pack_start(self.lbl_update_status, False, False, 0)
        row4.pack_start(r4_left, True, True, 0)

        btn_update = Gtk.Button(label=t("check_update", self.lang))
        btn_update.set_valign(Gtk.Align.CENTER)
        btn_update.connect("clicked", self.on_check_update_clicked)
        row4.pack_end(btn_update, False, False, 0)
        page.pack_start(row4, False, False, 0)

        # 5. Save Button
        bottom_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        btn_save = Gtk.Button(label=t("save_settings", self.lang))
        btn_save.connect("clicked", self.on_save_settings_clicked)
        bottom_box.pack_end(btn_save, False, False, 0)
        page.pack_start(bottom_box, False, False, 0)

        self.stack.add_named(page, "settings")

    def build_dev_page(self):
        page = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
        page.get_style_context().add_class("content-area")

        title = Gtk.Label(xalign=0, label="Công cụ Dev & Lệnh nhanh" if self.lang == "vi" else "Developer CLI & Tools")
        title.get_style_context().add_class("page-title")
        page.pack_start(title, False, False, 0)

        grid = Gtk.Grid()
        grid.set_column_spacing(10)
        grid.set_row_spacing(10)
        page.pack_start(grid, False, False, 0)

        btn_doc = Gtk.Button(label="502drive doctor (Terminal)")
        btn_doc.connect("clicked", self.tray.run_doctor)
        grid.attach(btn_doc, 0, 0, 1, 1)

        btn_run_embedded = Gtk.Button(label="Kiểm tra tại đây (Embedded Check)")
        btn_run_embedded.connect("clicked", self.on_run_embedded_doctor)
        grid.attach(btn_run_embedded, 1, 0, 1, 1)

        btn_logs = Gtk.Button(label="Live Logs (journalctl)")
        btn_logs.connect("clicked", self.tray.view_logs)
        grid.attach(btn_logs, 0, 1, 1, 1)

        btn_backup = Gtk.Button(label="Backup State DB")
        btn_backup.connect("clicked", self.tray.run_backup)
        grid.attach(btn_backup, 1, 1, 1, 1)

        btn_conf = Gtk.Button(label="config.toml (Edit)")
        btn_conf.connect("clicked", self.tray.edit_config)
        grid.attach(btn_conf, 0, 2, 1, 1)

        btn_rep = Gtk.Button(label="Reports Folder")
        btn_rep.connect("clicked", self.tray.open_reports)
        grid.attach(btn_rep, 1, 2, 1, 1)

        # Embedded console viewer for doctor results
        lbl_diag = Gtk.Label(xalign=0, label="Kết quả chẩn đoán nhanh (Quick Diagnostics):")
        lbl_diag.get_style_context().add_class("card-title")
        lbl_diag.set_margin_top(10)
        page.pack_start(lbl_diag, False, False, 0)

        scrolled_text = Gtk.ScrolledWindow()
        scrolled_text.set_min_content_height(140)
        scrolled_text.get_style_context().add_class("console-box")
        self.text_doctor = Gtk.TextView()
        self.text_doctor.set_editable(False)
        self.text_doctor.set_cursor_visible(False)
        self.text_doctor.get_buffer().set_text("Bấm 'Kiểm tra tại đây' để chạy doctor trực tiếp..." if self.lang == "vi" else "Click 'Embedded Check' to run doctor...")
        scrolled_text.add(self.text_doctor)
        page.pack_start(scrolled_text, True, True, 0)

        self.stack.add_named(page, "dev")

    def on_run_embedded_doctor(self, btn):
        buf = self.text_doctor.get_buffer()
        buf.set_text("Đang chạy 502drive doctor..." if self.lang == "vi" else "Running 502drive doctor...")
        btn.set_sensitive(False)

        def worker():
            ok, out, err = run_cmd("502drive doctor")
            res = out if ok else (err or out or "Error")
            def done():
                buf.set_text(res)
                btn.set_sensitive(True)
            GLib.idle_add(done)

        threading.Thread(target=worker, daemon=True).start()

    def load_config_data(self):
        if CONFIG_PATH.exists():
            try:
                with open(CONFIG_PATH, "rb") as f:
                    return tomllib.load(f)
            except Exception:
                pass
        return {}

    def on_save_settings_clicked(self, _):
        if not CONFIG_PATH.exists():
            return
        try:
            auto_confirm = "true" if self.sw_auto_confirm.get_active() else "false"
            with open(CONFIG_PATH, "r", encoding="utf-8") as f:
                lines = f.readlines()
            new_lines = []
            for line in lines:
                if line.strip().startswith("auto_confirm_clone"):
                    new_lines.append(f"auto_confirm_clone = {auto_confirm}\n")
                else:
                    new_lines.append(line)
            with open(CONFIG_PATH, "w", encoding="utf-8") as f:
                f.writelines(new_lines)
            run_cmd(f"systemctl --user restart {SERVICE_NAME}")
            notify("502Drive", t("settings_saved", self.lang))
        except Exception as e:
            notify("502Drive", f"Error saving settings: {e}")

    def on_startup_toggled(self, switch, gparam):
        active = switch.get_active()
        self.tray.set_autostart_state(active)

    def on_check_update_clicked(self, btn):
        self.lbl_update_status.set_text(t("checking", self.lang))
        btn.set_sensitive(False)

        def worker():
            msg = t("up_to_date", self.lang, ver=APP_VERSION)
            try:
                req = urllib.request.Request(
                    "https://api.github.com/repos/GiaHung07/502Drive/releases/latest",
                    headers={"User-Agent": "502Drive"}
                )
                with urllib.request.urlopen(req, timeout=4) as resp:
                    data = json.loads(resp.read().decode())
                    tag = data.get("tag_name")
                    if tag and tag != APP_VERSION:
                        msg = t("new_version_found", self.lang, ver=tag)
            except Exception:
                msg = t("up_to_date", self.lang, ver=APP_VERSION)

            def done():
                self.lbl_update_status.set_text(msg)
                btn.set_sensitive(True)
            GLib.idle_add(done)

        threading.Thread(target=worker, daemon=True).start()

    def refresh_data(self):
        active = self.tray.is_service_active()
        google_email = self.tray.get_google_account_email()

        if google_email:
            self.c1_email.set_text(f"Email: {google_email} (Đã kết nối / Connected)")
            self.btn_revoke.set_sensitive(True)
        else:
            self.c1_email.set_text("Chưa kết nối tài khoản Google" if self.lang == "vi" else "Google Drive Not Connected")
            self.btn_revoke.set_sensitive(False)

        if active:
            self.c2_status.set_text("Dịch vụ Bot: Đang chạy (Active)" if self.lang == "vi" else "Bot Service: Running (Active)")
            self.lbl_sidebar_status.set_text("Bot: Đang chạy" if self.lang == "vi" else "Bot: Running")
            self.lbl_sidebar_status.get_style_context().remove_class("status-pill-warn")
            self.lbl_sidebar_status.get_style_context().add_class("status-pill-ok")
        else:
            self.c2_status.set_text("Dịch vụ Bot: Đang dừng (Stopped)" if self.lang == "vi" else "Bot Service: Stopped")
            self.lbl_sidebar_status.set_text("Bot: Đang dừng" if self.lang == "vi" else "Bot: Stopped")
            self.lbl_sidebar_status.get_style_context().remove_class("status-pill-ok")
            self.lbl_sidebar_status.get_style_context().add_class("status-pill-warn")

        # Destination info
        dest_text = "Chưa đặt (Dùng /set_destination trên Telegram)" if self.lang == "vi" else "Not set (Use /set_destination on Telegram)"
        if DB_PATH.exists():
            try:
                conn = sqlite3.connect(f"file:{DB_PATH}?mode=ro", uri=True)
                c = conn.cursor()
                c.execute("SELECT label, destination_parent_id FROM destination_profiles WHERE is_default = 1")
                row = c.fetchone()
                conn.close()
                if row:
                    dest_text = f"Mặc định: {row[0]} ({row[1]})"
            except Exception:
                pass
        self.c3_desc.set_text(dest_text)


# ── Silky-Smooth Compact Tray Controller ───────────────────────────────────────
class DriveTray:
    def __init__(self):
        self.last_status = None
        self.last_google_email = None
        self.lang = self.get_config_language()
        self.app_window = None

        icon = ICON_ACTIVE if self.is_service_active() else ICON_INACTIVE
        self.indicator = AyatanaAppIndicator3.Indicator.new(
            APP_ID,
            icon,
            AyatanaAppIndicator3.IndicatorCategory.APPLICATION_STATUS,
        )
        # Set icon theme path so indicator can resolve icon names from ~/.local/share/icons
        try:
            self.indicator.set_icon_theme_path(ICON_THEME_PATH)
        except Exception:
            pass
        self.indicator.set_status(AyatanaAppIndicator3.IndicatorStatus.ACTIVE)

        self.menu = Gtk.Menu()
        self.indicator.set_menu(self.menu)

        self.build_menu()

        # Record PID and attach SIGUSR1 IPC
        try:
            PID_FILE.write_text(str(os.getpid()))
            signal.signal(signal.SIGUSR1, lambda s, f: GLib.idle_add(self.show_dashboard))
        except Exception:
            pass

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
        if self.app_window is None:
            self.app_window = DriveAppWindow(self)
        self.app_window.refresh_data()
        self.app_window.show_all()
        self.app_window.present()

    def build_menu(self):
        # Clear once
        for child in self.menu.get_children():
            self.menu.remove(child)

        lang = self.lang
        active = self.is_service_active()
        google_email = self.get_google_account_email()
        self.last_status = active
        self.last_google_email = google_email

        # 1. Primary Action: Mở Bảng điều khiển... (Compact, like 502Print!)
        dash_item = make_item(
            t("open_dashboard", lang),
            icon_name="document-open-symbolic",
            callback=self.show_dashboard,
        )
        self.menu.append(dash_item)

        # 2. Telegram Bot Link (Clean & short, no bloated text!)
        tg_item = make_item(
            t("open_telegram", lang),
            icon_name="send-to-symbolic",
            callback=self.open_telegram,
        )
        self.menu.append(tg_item)

        self.menu.append(Gtk.SeparatorMenuItem())

        # 3. Google Account Submenu (Compact text to prevent shell resizing pop)
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

        # 4. Bot Service Submenu
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

        # 5. Admin & Data Submenu (Clean & compact labels)
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

        # 8. Quit (Clean & compact, identical to 502Print)
        quit_item = make_item(
            t("quit", lang),
            icon_name="application-exit-symbolic",
            callback=self.quit_app,
        )
        self.menu.append(quit_item)

        self.menu.show_all()

    def update_status(self):
        # In-place updates to completely prevent stuttering
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

        if self.app_window and self.app_window.is_visible():
            self.app_window.refresh_data()

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

    # Load CSS styling
    provider = Gtk.CssProvider()
    provider.load_from_data(APP_CSS.encode())
    Gtk.StyleContext.add_provider_for_screen(
        Gdk.Screen.get_default(),
        provider,
        Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
    )

    # If GUI requested and daemon is already running, notify daemon and exit
    show_gui_arg = len(sys.argv) > 1 and any(arg in sys.argv[1:] for arg in ["--window", "--gui", "gui", "open"])
    if show_gui_arg and PID_FILE.exists():
        try:
            running_pid = int(PID_FILE.read_text().strip())
            os.kill(running_pid, 0)
            os.kill(running_pid, signal.SIGUSR1)
            sys.exit(0)
        except (ProcessLookupError, ValueError):
            PID_FILE.unlink(missing_ok=True)
        except Exception:
            pass

    tray = DriveTray()

    # Always open the dashboard window on launch (from app launcher, dock, search)
    # Only skip if running as background service (--no-window flag)
    no_window = any(arg in sys.argv[1:] for arg in ["--no-window", "--background", "--service"])
    if not no_window:
        # Small delay to ensure GTK is ready before showing window
        GLib.timeout_add(200, tray.show_dashboard)

    Gtk.main()


if __name__ == "__main__":
    main()
