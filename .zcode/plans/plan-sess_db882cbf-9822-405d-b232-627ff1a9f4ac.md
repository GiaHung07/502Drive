# PLAN NÂNG CẤP LỚN — 502DRIVE (5 PHASE)

**Bối cảnh đã khảo sát:** CI đỏ trên GitHub vì `cargo fmt --check` fail ở commit cũ (đã được sửa trong 5 commit local chưa push) · 0 GitHub Release (release.yml hỏng ở bước Windows) · Dockerfile **không build được** (thiếu workspace member `src-tauri`) · 6 file docs nội bộ cần gỡ khỏi history · engine còn 5 điểm tốn RAM/CPU rõ rệt · GUI chưa tạo được watch/clone.

---

## PHASE 0 — DỌN SẠCH HISTORY & XUẤT BẢN (làm đầu tiên)
1. Sửa `packaging/502drive-tray.service` (còn hardcode `/home/admin` → dùng `%h`).
2. Gỡ file nội bộ khỏi **toàn bộ history** bằng `git filter-repo`: xóa `.ai/`, `.agents/`, `docs/ai-handoff.md`, `progress.md`, `private-local.vi.md`, `reference-analysis.md`, `SYSTEM_ARCHITECTURE_AND_AI_AUDIT_DOSSIER.md`, `PROJECT_SPEC.vi.md`; replace-text `/home/admin` → `~`. Force-push main (repo do AI viết từ đầu, không ai phụ thuộc → an toàn theo xác nhận của bạn). Giữ lại các docs công khai có giá trị.
3. Push → CI sẽ xanh lại (lỗi fmt đã được sửa trong các commit này). Thêm bước **gitleaks** vào `ci.yml` để chặn secret vĩnh viễn.

## PHASE 1 — VIẾT LẠI DOCS CHUẨN OSS
- **README.md + README.vi.md** viết lại (giữ lockstep song ngữ): GIF demo có sẵn trong `docs/assets/`, bảng "what works" trung thực, mục **Sync semantics** (1 chiều source→destination, deletion policy rõ ràng), bảng config đầy đủ **TOML + biến môi trường `GDCLONE__*`**, mục giới thiệu **desktop GUI**, FAQ, docs index.
- Cấu trúc docs mới: `oauth-setup.md` (mở rộng walkthrough Google Cloud), `install-linux.md`, `install-windows.md`, `install-docker.md`, `sync-semantics.md`, gộp `recovery-model.md` vào `architecture.md`, mở rộng `troubleshooting.md`, `SECURITY.md` thêm kênh báo cáo riêng.
- **PRIVACY.md**: sửa claim sai — token mã hóa bằng file `master.key` 0600, không phải system keyring.
- Chụp **screenshot GUI thật** (dark + light) đưa vào README qua browser.

## PHASE 2 — RELEASE v0.2.0 + DOCKER/VPS
- **Version single-source**: script `scripts/set-version.sh` cập nhật đồng loạt 5 chỗ (2 Cargo.toml, tauri.conf.json, package.json) → bump 0.2.0.
- **Sửa release.yml**: job Windows hỏng ở `package-release.ps1` (restructure: tách bước build, chỉ đóng gói exe đã build + sha256); **thêm job build GUI bundle** — Linux (deb + AppImage qua `tauri build`, cài webkit2gtk deps) và Windows (.msi/.exe installer).
- **Sửa Docker/VPS**: Dockerfile COPY đủ workspace (nguyên nhân build fail), đồng bộ port OAuth 51000–51100, thêm HEALTHCHECK, `docker-compose.yml` hỗ trợ config qua biến `GDCLONE__*` (khắc phục OAuth headless: user điền token/env thay vì mở browser), viết `docs/install-docker.md`. Verify `docker build` nếu sandbox có docker; không thì review kỹ + bạn chạy xác nhận.
- Xóa remote tag `v0.1.0` cũ (đang trỏ sai commit) → tag **v0.2.0** trên commit sạch → release tự publish với artifacts. Verify trang Releases.

## PHASE 3 — SIÊU TỐI ƯU RAM/CPU + REALTIME (engine)
1. **Dùng chung 1 `DriveClient`** thay vì 13 chỗ tự tạo reqwest pool riêng; bỏ clone `AppConfig` + `TokenManager` mới per-cursor-per-poll trong poller.
2. **Chặn `scan_missing_children` chạy mỗi chu kỳ ~20s mỗi watch** (chi phí API + CPU lớn nhất) → chỉ chạy sau catch-up hoặc theo interval config.
3. **Batch DB calls của dispatcher** vào 1 transaction/batch; bỏ N+1 trong `commit_change_page` (dùng `last_insert_rowid`).
4. **Slim `change_events`**: lưu các field cần thiết thay vì full `file_json` (migration 0003) + prune event đã consume mạnh tay hơn → giảm RAM/ổ đĩa đáng kể.
5. **Realtime nhanh gấp ~10 lần khi active**: notify channel poller→dispatcher để dispatch **ngay sau khi poll có event** (hiện phải chờ trọn interval); hạ `active_poll_seconds` default xuống 10s.
- Mỗi mục có test/compile check; smoke-test daemon khởi động.

## PHASE 4 — TÍNH NĂNG TIỆN LỢI CHO USER
- **Engine**: `CloneRequest` thêm rename-before-clone + destination override; chọn duplicate-policy lúc clone; watch filters/exclusions (loại file/pattern).
- **GUI (điểm gap số 1)**: lệnh Tauri `create_watch`, `start_clone`, `retry_job`, `list_children` (folder browser), `unwatch`/`watch_policy`; frontend: **dán link Drive → clone ngay tại GUI** (chọn đích + đổi tên), modal tạo watch, nút Retry job, sửa `backlog_count` tính sai, watch card thêm hành động.
- **Telegram**: đăng ký **đầy đủ command menu** (hiện chỉ 11/~25), `/help` hoàn chỉnh, giải thích lỗi watch bằng `format_drive_error`, thông báo hoàn tất watch event (batch, chống spam).
- **Roadmap phase sau (không làm round này)**: two-way sync + conflict rules, Drive push webhook cho VPS (realtime <1s), multi-account GUI, cross-device backup trong GUI.

**Mỗi phase xong: build + test + commit riêng. Cuối cùng: push tất cả, verify CI xanh + Release trang GitHub.**
