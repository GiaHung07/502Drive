# SPEC: `gdclone-bot` — Local-first Telegram Google Drive Clone & Watch Bot

> **Master prompt cho IDE agent (TRAE/Windsurf/Codex/Claude Code).**  
> Viết mới theo hướng clean-room; chỉ học kiến trúc và hành vi từ repo tham chiếu.  
> Target chính: **CachyOS/Arch Linux**. Target thứ hai: **Windows 10/11**.  
> **Không yêu cầu VPS, Docker, public IP, domain hoặc webhook.**

---

# 0. Vai trò và nguyên tắc thực thi của agent

Bạn là **Principal Backend Engineer + Distributed Systems Engineer + Security Engineer + DevOps Engineer** chịu trách nhiệm thiết kế và triển khai dự án này đến trạng thái chạy được.

Không chỉ đưa ra kế hoạch. Phải:

1. Kiểm tra repository hiện tại.
2. Clone và phân tích repo tham chiếu.
3. Đối chiếu với tài liệu chính thức mới nhất của Google Drive API, Google OAuth, Telegram Bot API và các crate Rust được chọn.
4. Ghi lại các quyết định kỹ thuật quan trọng.
5. Triển khai theo từng phase.
6. Chạy formatter, linter, test, migration và build.
7. Sửa toàn bộ lỗi tái hiện được.
8. Để lại project có thể chạy thật trên CachyOS.
9. Giữ Windows build xanh từ đầu và kiểm thử thật trên Windows ở phase riêng.

Báo cáo tiến độ và báo cáo cuối bằng **tiếng Việt**. Giữ nguyên tiếng Anh đối với:

- source code;
- command;
- file path;
- API name;
- crate name;
- function/type name;
- log;
- error message.

Không được:

- viết pseudocode thay cho core implementation;
- để TODO ở luồng clone, checkpoint, retry, recovery hoặc watch;
- giả lập thành công khi chưa chạy test;
- né quota, quyền truy cập hoặc cơ chế chống abuse của Google;
- xoay Service Account để vượt hạn mức;
- download rồi upload lại file thông thường khi `files.copy` có thể xử lý server-side;
- copy nguyên repo hoặc code GPL vào project mới;
- đoán license, API behavior hoặc quota.

Nếu thiếu credential thật, vẫn phải hoàn thiện code, mock test, setup guide và validation; chỉ đánh dấu rõ bước end-to-end nào chưa thể chạy.

---

# 1. Mục tiêu sản phẩm

## 1.1. One-shot clone

Người dùng có thể:

1. Đăng nhập Google một lần trên chính máy chạy bot.
2. Dán Google Drive file/folder URL vào Telegram.
3. Kiểm tra metadata nguồn.
4. Chọn folder đích trong My Drive hoặc Shared Drive.
5. Clone file hoặc toàn bộ cây folder.
6. Theo dõi tiến trình trong một Telegram message.
7. Pause, resume, cancel và retry.
8. Tự tiếp tục sau khi app/máy bị restart.
9. Nhận report JSON/CSV cuối job.

## 1.2. Watch/Sync

Sau khi one-shot clone ổn định, người dùng có thể tạo một watch subscription:

- file mới trong cây nguồn → clone vào cây đích;
- rename nguồn → rename bản đích;
- move trong cây watch → move bản đích theo mapping;
- move ra ngoài cây watch → giữ bản đích, đánh dấu detached;
- move trở lại → nối lại mapping cũ, không tạo bản sao trùng;
- source bị trash/xóa/mất quyền → giữ bản đích, cảnh báo một lần;
- nội dung file thay đổi → áp dụng policy đã chọn;
- mọi cursor, event và mapping sống sót qua restart.

## 1.3. Non-goals của v1

Không làm:

- web dashboard;
- Telegram webhook;
- multi-tenant SaaS;
- public OAuth callback;
- torrent/leech/yt-dlp/NZB;
- upload Telegram → Drive;
- permission cloning đầy đủ;
- ownership transfer;
- xin quyền/truy cập hộ người dùng khi nguồn bị chặn; bot chỉ báo không dùng
  được và ghi rõ vào report;
- service-account rotation;
- quota bypass;
- viewer-only / download-disabled bypass, browser scraping, screenshot/PDF
  capture, Apple Files/iOS provider interception, MITM, hoặc bất kỳ cách nào
  nhằm vượt policy của owner;
- content diff theo từng byte/trang/cell;
- realtime tuyệt đối.

Telegram là UI chính. Dashboard localhost-only có thể xem xét ở v2.

---

# 2. Quyết định kiến trúc

| Hạng mục | Quyết định | Lý do |
|---|---|---|
| Ngôn ngữ | Rust stable, ưu tiên edition 2024 | Một executable, type safety, concurrency và error handling tốt |
| Async runtime | `tokio` | Runtime chính, task cancellation, timer, channel, semaphore |
| Telegram | `teloxide`, long polling | Không cần public endpoint; Telegram `getUpdates` không chạy cùng webhook |
| Google OAuth | `oauth2`, Authorization Code + PKCE, Desktop loopback | Đúng mô hình desktop/local; listener chỉ bind `127.0.0.1` |
| OAuth UX | CLI tương tác `gdclone-bot auth login` | Loopback redirect chỉ hoạt động trên chính máy mở browser; không giả vờ `/connect` từ điện thoại sẽ hoạt động |
| HTTP | `reqwest` async với `rustls-tls`, redirect disabled cho token client | Tránh phụ thuộc OpenSSL hệ thống và giảm SSRF risk trong OAuth client |
| Drive API | REST v3 trực tiếp | Kiểm soát fields, pagination, resource key, Shared Drive và retry |
| Database | SQLite qua `tokio-rusqlite`/DB actor; `rusqlite` bundled | Không block Tokio worker; không cần DB server |
| Journal | SQLite WAL + foreign keys + busy timeout | Crash recovery và đọc/ghi local ổn định |
| Secret storage | `SecretStore` abstraction; random master key/OS protection | Không derive key chỉ từ machine-id vì machine-id không phải secret |
| Logging | `tracing` + rolling file + stdout/journald | Structured log, correlation ID, redaction |
| Linux startup | `systemd --user`; `loginctl enable-linger` là tùy chọn để chạy từ boot | User service bình thường phụ thuộc user manager/login |
| Windows startup | Task Scheduler at logon là mặc định; Windows Service là advanced option | Giữ đúng user profile/credential context; service không được mặc định chạy LocalSystem |
| Packaging | Native binary; Docker không bắt buộc | Local-first |
| Static linking | Không tuyên bố Linux binary “fully static” mặc định | Musl static build là tùy chọn; glibc build thường không hoàn toàn static |

## 2.1. Kiến trúc runtime

```text
Telegram Bot API (long polling)
              │
              ▼
      Telegram dispatcher
              │
       command/event layer
              │
              ▼
       Application services
      ┌───────┴────────┐
      │                │
One-shot engine    Watch engine
      │                │
      └───────┬────────┘
              ▼
       Durable scheduler
      ┌───────┴─────────────┐
      │                     │
 Drive API client      SQLite state
      │             (jobs, mappings,
      │              intents, cursors,
      │              raw changes)
      ▼
Google Drive API v3
```

## 2.2. Các invariant bắt buộc

1. Không có thao tác Drive write nào chạy mà chưa có durable operation intent.
2. Không đánh dấu item `done` trước khi có `dest_item_id` hợp lệ.
3. Không advance change cursor trước khi raw change events của page đó được commit.
4. Không dựa vào “exactly once” giữa SQLite và Google Drive; phải đạt **effectively-once** bằng idempotency key, `appProperties` và reconciliation.
5. Không block Tokio runtime bằng thao tác SQLite đồng bộ.
6. Không chạy số lượng request không giới hạn.
7. Cancellation chỉ cooperative giữa các API call; request đang bay có thể vẫn hoàn thành và phải được reconcile.
8. Source deletion không tự động xóa destination trong policy mặc định.
9. Watch subscription là đối tượng sống lâu; job là một lần thực thi hữu hạn. Không nhét watch vào `jobs.mode` rồi dùng chung lifecycle một cách mơ hồ.
10. Một Google account + một corpus chỉ có **một change poller**, không một poller cho mỗi watched folder.

---

# 3. Cấu trúc project

```text
gdclone-bot/
├── Cargo.toml
├── Cargo.lock
├── config.sample.toml
├── .gitignore
├── README.md
├── SECURITY.md
├── THIRD_PARTY_NOTICES.md
├── docs/
│   ├── architecture.md
│   ├── reference-analysis.md
│   ├── google-oauth-setup.md
│   ├── cachyos-setup.md
│   ├── windows-setup.md
│   ├── watch-semantics.md
│   ├── recovery-model.md
│   └── troubleshooting.md
├── references/                         # gitignored, read-only research
├── migrations/
│   ├── 0001_core.sql
│   ├── 0002_watch.sql
│   └── 0003_indexes.sql
├── packaging/
│   ├── gdclone-bot.service
│   └── windows-task.ps1
├── scripts/
│   ├── setup-cachyos.sh
│   ├── install-systemd-user.sh
│   ├── uninstall-systemd-user.sh
│   ├── setup-windows.ps1
│   ├── install-windows-task.ps1
│   ├── backup.sh
│   └── restore.sh
├── src/
│   ├── main.rs
│   ├── app.rs
│   ├── error.rs
│   ├── config.rs
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   ├── service.rs
│   │   └── doctor.rs
│   ├── telegram/
│   │   ├── mod.rs
│   │   ├── commands.rs
│   │   ├── handlers.rs
│   │   ├── dialogue.rs
│   │   ├── keyboards.rs
│   │   ├── authorization.rs
│   │   └── progress.rs
│   ├── drive/
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   ├── token_manager.rs
│   │   ├── client.rs
│   │   ├── request.rs
│   │   ├── fields.rs
│   │   ├── errors.rs
│   │   ├── pacer.rs
│   │   ├── links.rs
│   │   ├── resource_keys.rs
│   │   ├── changes.rs
│   │   └── types.rs
│   ├── engine/
│   │   ├── mod.rs
│   │   ├── job.rs
│   │   ├── discovery.rs
│   │   ├── traversal.rs
│   │   ├── copy.rs
│   │   ├── shortcut.rs
│   │   ├── duplicate.rs
│   │   ├── scheduler.rs
│   │   ├── operation.rs
│   │   └── reconcile.rs
│   ├── watch/
│   │   ├── mod.rs
│   │   ├── poller.rs
│   │   ├── event_log.rs
│   │   ├── dispatcher.rs
│   │   ├── membership.rs
│   │   ├── classifier.rs
│   │   ├── applier.rs
│   │   └── retention.rs
│   ├── state/
│   │   ├── mod.rs
│   │   ├── db.rs
│   │   ├── migrations.rs
│   │   ├── models.rs
│   │   └── repositories/
│   │       ├── jobs.rs
│   │       ├── items.rs
│   │       ├── mappings.rs
│   │       ├── intents.rs
│   │       ├── watches.rs
│   │       └── cursors.rs
│   ├── secrets/
│   │   ├── mod.rs
│   │   ├── file_store.rs
│   │   ├── linux.rs
│   │   └── windows.rs
│   ├── report/
│   │   └── mod.rs
│   └── platform/
│       ├── mod.rs
│       ├── linux.rs
│       └── windows.rs
└── tests/
    ├── link_parser.rs
    ├── resource_key.rs
    ├── retry.rs
    ├── state_machine.rs
    ├── idempotency.rs
    ├── recovery.rs
    ├── watch_cursor.rs
    ├── watch_membership.rs
    └── shared_drive.rs
```

Không tạo “god module”. Telegram handler chỉ parse/authorize/dispatch, không chứa Drive business logic.

---

# 4. Repo tham chiếu và quy tắc clean-room

Trước khi code:

```bash
mkdir -p references

git clone --depth 1 \
  https://github.com/Zyforaa/Google-Drive-Bot-Cloudflare.git \
  references/google-drive-bot-cloudflare

git clone --depth 1 \
  https://github.com/anasty17/mirror-leech-telegram-bot.git \
  references/mirror-leech-telegram-bot

git clone --depth 1 \
  https://github.com/TheCaduceus/CloneBot_V2.git \
  references/clonebot-v2

git clone --depth 1 \
  https://github.com/nithilamandiw/telegram-drive-bot.git \
  references/telegram-drive-bot

git clone --depth 1 \
  https://github.com/rclone/rclone.git \
  references/rclone

git clone --depth 1 \
  https://github.com/googleworkspace/python-samples.git \
  references/google-workspace-python-samples
```

Thêm:

```gitignore
references/
config.toml
*.db
*.db-wal
*.db-shm
data/
reports/
logs/
master.key
```

Agent phải tạo `docs/reference-analysis.md` với bảng:

| Repo | Commit SHA | License | Tính năng liên quan | Concept học | Code không được lấy | Rủi ro | Thành phần lỗi thời | Quyết định |
|---|---|---|---|---|---|---|---|---|

Quy tắc:

- `anasty17/mirror-leech-telegram-bot` là GPL-3.0: chỉ học behavior/UX, không copy code.
- License còn lại phải đọc từ repository thực tế, không đoán.
- Ưu tiên clean-room.
- Nếu có code adapt, ghi đầy đủ vào `THIRD_PARTY_NOTICES.md`:
  - URL;
  - commit SHA;
  - file path;
  - license;
  - đoạn/chức năng được adapt;
  - thay đổi đã thực hiện.
- Tài liệu chính thức mới nhất luôn có ưu tiên cao hơn repo cũ.

---

# 5. Database và concurrency model

## 5.1. SQLite runtime

Dùng `tokio-rusqlite` hoặc một DB actor tương đương để mọi `rusqlite` call chạy trên dedicated blocking thread.

Khi mở DB:

```sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 5000;
```

Không tạo pool nhiều writer không kiểm soát. Một serialized writer path là đủ cho local app.

Mỗi migration chạy transactionally và được ghi version.

Timestamps lưu bằng Unix milliseconds (`INTEGER`) để sort/index dễ, convert sang UTC ở boundary.

## 5.2. Core schema

```sql
CREATE TABLE authorized_users (
    telegram_user_id INTEGER PRIMARY KEY,
    role TEXT NOT NULL CHECK(role IN ('owner', 'operator')),
    enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0, 1)),
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE google_accounts (
    id TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    email TEXT,
    refresh_token_ciphertext BLOB NOT NULL,
    refresh_token_nonce BLOB,
    secret_backend TEXT NOT NULL,
    scopes_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN
        ('connected', 'reconnect_required', 'revoked', 'disabled')),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE jobs (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK(kind IN
        ('one_shot', 'watch_initial_clone', 'watch_apply', 'reconcile')),
    telegram_user_id INTEGER NOT NULL,
    chat_id INTEGER NOT NULL,
    google_account_id TEXT NOT NULL REFERENCES google_accounts(id),
    source_root_id TEXT NOT NULL,
    source_resource_key TEXT,
    source_drive_id TEXT,
    destination_parent_id TEXT NOT NULL,
    destination_drive_id TEXT,
    status TEXT NOT NULL CHECK(status IN (
        'queued', 'discovering', 'running', 'pausing', 'paused',
        'cancelling', 'cancelled', 'recovering',
        'completed', 'partially_completed', 'failed'
    )),
    duplicate_policy TEXT NOT NULL CHECK(duplicate_policy IN
        ('keep_both', 'skip_same_source', 'replace_safe')),
    shortcut_policy TEXT NOT NULL CHECK(shortcut_policy IN
        ('preserve', 'remap_internal', 'resolve')),
    total_discovered INTEGER NOT NULL DEFAULT 0,
    completed_items INTEGER NOT NULL DEFAULT 0,
    failed_items INTEGER NOT NULL DEFAULT 0,
    skipped_items INTEGER NOT NULL DEFAULT 0,
    progress_message_id INTEGER,
    error_summary TEXT,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE INDEX idx_jobs_status ON jobs(status, updated_at_ms);
CREATE INDEX idx_jobs_user ON jobs(telegram_user_id, created_at_ms);

CREATE TABLE traversal_folders (
    job_id TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    source_folder_id TEXT NOT NULL,
    source_resource_key TEXT,
    destination_folder_id TEXT NOT NULL,
    source_parent_id TEXT,
    scan_state TEXT NOT NULL CHECK(scan_state IN
        ('pending', 'listing', 'done', 'failed')),
    next_page_token TEXT,
    last_error TEXT,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY(job_id, source_folder_id)
);

CREATE TABLE job_items (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    source_item_id TEXT NOT NULL,
    source_resource_key TEXT,
    source_parent_id TEXT,
    destination_parent_id TEXT NOT NULL,
    destination_item_id TEXT,
    mime_type TEXT NOT NULL,
    item_kind TEXT NOT NULL CHECK(item_kind IN
        ('binary', 'google_native', 'folder', 'shortcut', 'unknown')),
    source_name TEXT NOT NULL,
    size_bytes INTEGER,
    source_version TEXT,
    source_modified_time TEXT,
    source_md5_checksum TEXT,
    generation INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL CHECK(status IN (
        'discovered', 'ready', 'copying', 'done',
        'failed', 'skipped_duplicate', 'skipped_unsupported',
        'cancelled'
    )),
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error_code TEXT,
    last_error_message TEXT,
    completed_at_ms INTEGER,
    UNIQUE(job_id, source_item_id, generation)
);

CREATE INDEX idx_job_items_work
ON job_items(job_id, status, id);

CREATE TABLE source_mappings (
    scope_type TEXT NOT NULL CHECK(scope_type IN ('job', 'watch')),
    scope_id TEXT NOT NULL,
    source_item_id TEXT NOT NULL,
    destination_item_id TEXT NOT NULL,
    source_parent_id TEXT,
    destination_parent_id TEXT,
    mime_type TEXT NOT NULL,
    source_name TEXT NOT NULL,
    source_version TEXT,
    source_modified_time TEXT,
    source_md5_checksum TEXT,
    mapping_state TEXT NOT NULL CHECK(mapping_state IN
        ('active', 'detached', 'source_removed', 'replaced')),
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY(scope_type, scope_id, source_item_id)
);

CREATE INDEX idx_mapping_destination
ON source_mappings(destination_item_id);

CREATE TABLE operation_intents (
    id TEXT PRIMARY KEY,
    idempotency_key TEXT NOT NULL UNIQUE,
    job_id TEXT REFERENCES jobs(id) ON DELETE CASCADE,
    watch_id TEXT,
    operation_type TEXT NOT NULL CHECK(operation_type IN (
        'create_folder', 'copy_file', 'create_shortcut',
        'rename', 'move', 'trash', 'restore_mapping'
    )),
    source_item_id TEXT,
    destination_parent_id TEXT,
    destination_item_id TEXT,
    request_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN (
        'planned', 'executing', 'applied',
        'cleanup_pending', 'failed'
    )),
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);
```

## 5.3. Watch schema

```sql
CREATE TABLE change_cursors (
    id TEXT PRIMARY KEY,
    google_account_id TEXT NOT NULL REFERENCES google_accounts(id),
    corpus_kind TEXT NOT NULL CHECK(corpus_kind IN ('user', 'shared_drive')),
    drive_id TEXT,
    current_page_token TEXT NOT NULL,
    last_event_sequence INTEGER NOT NULL DEFAULT 0,
    next_poll_at_ms INTEGER NOT NULL,
    last_success_at_ms INTEGER,
    consecutive_error_count INTEGER NOT NULL DEFAULT 0,
    UNIQUE(google_account_id, corpus_kind, drive_id)
);

CREATE TABLE change_events (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    cursor_id TEXT NOT NULL REFERENCES change_cursors(id) ON DELETE CASCADE,
    request_page_token TEXT NOT NULL,
    ordinal_in_page INTEGER NOT NULL,
    file_id TEXT NOT NULL,
    removed INTEGER NOT NULL CHECK(removed IN (0, 1)),
    file_json TEXT,
    received_at_ms INTEGER NOT NULL,
    UNIQUE(cursor_id, request_page_token, ordinal_in_page)
);

CREATE INDEX idx_change_events_cursor_sequence
ON change_events(cursor_id, sequence);

CREATE TABLE watch_subscriptions (
    id TEXT PRIMARY KEY,
    google_account_id TEXT NOT NULL REFERENCES google_accounts(id),
    cursor_id TEXT NOT NULL REFERENCES change_cursors(id),
    telegram_user_id INTEGER NOT NULL,
    chat_id INTEGER NOT NULL,
    source_root_id TEXT NOT NULL,
    source_resource_key TEXT,
    source_drive_id TEXT,
    destination_root_id TEXT NOT NULL,
    destination_drive_id TEXT,
    status TEXT NOT NULL CHECK(status IN (
        'initializing', 'catching_up', 'active',
        'paused', 'degraded', 'needs_reconcile', 'stopped'
    )),
    content_update_policy TEXT NOT NULL CHECK(content_update_policy IN (
        'versioned_copy', 'replace_copy', 'manual_confirmation'
    )),
    deletion_policy TEXT NOT NULL CHECK(deletion_policy IN (
        'preserve_destination', 'manual_confirmation'
    )),
    move_out_policy TEXT NOT NULL CHECK(move_out_policy IN (
        'detach', 'keep_following'
    )),
    baseline_sequence INTEGER NOT NULL,
    last_consumed_sequence INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE watch_event_applications (
    watch_id TEXT NOT NULL REFERENCES watch_subscriptions(id) ON DELETE CASCADE,
    event_sequence INTEGER NOT NULL REFERENCES change_events(sequence) ON DELETE CASCADE,
    classification TEXT NOT NULL CHECK(classification IN (
        'new_item', 'content_changed', 'renamed', 'moved_inside',
        'moved_outside', 'moved_back', 'trashed_or_removed',
        'permission_lost', 'irrelevant', 'ambiguous'
    )),
    status TEXT NOT NULL CHECK(status IN (
        'pending', 'applying', 'applied', 'ignored', 'failed'
    )),
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY(watch_id, event_sequence)
);

Thêm bảng destination mặc định:

CREATE TABLE destination_profiles (
    id TEXT PRIMARY KEY,
    google_account_id TEXT NOT NULL REFERENCES google_accounts(id),
    label TEXT NOT NULL,
    destination_parent_id TEXT NOT NULL,
    destination_drive_id TEXT,
    destination_resource_key TEXT,
    is_default INTEGER NOT NULL DEFAULT 0 CHECK(is_default IN (0, 1)),
    last_validated_at_ms INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE UNIQUE INDEX idx_one_default_destination
ON destination_profiles(google_account_id)

## 5.4. Token storage

Không persist access token nếu không cần. Access token giữ trong memory cache và refresh theo singleflight.

Refresh token:

- mã hóa trước khi lưu;
- key không nằm trong DB;
- key không derive chỉ từ `/etc/machine-id` hoặc `MachineGuid`;
- Linux default: tạo random 256-bit master key, file permission `0600`;
- Windows default: DPAPI CurrentUser hoặc key file ACL chỉ current user;
- nếu Windows Service chạy account khác user đã auth thì phải từ chối startup với lỗi rõ ràng.

---

# 6. State machine và recovery

## 6.1. Job

```text
Queued
  └─> Discovering
        └─> Running
              ├─> Pausing -> Paused -> Running
              ├─> Cancelling -> Cancelled
              ├─> Completed
              ├─> PartiallyCompleted
              └─> Failed

App restart:
Discovering/Running/Pausing/Cancelling
  └─> Recovering
        └─> trạng thái hợp lệ tiếp theo sau reconciliation
```

Không cho transition tùy ý. Viết transition guard và unit test.

## 6.2. Item

```text
Discovered -> Ready -> Copying -> Done
                           ├──> Failed
                           ├──> SkippedDuplicate
                           ├──> SkippedUnsupported
                           └──> Cancelled
```

Khi restart:

- `Copying` không được reset mù thành `Ready`;
- trước tiên chạy reconciliation:
  1. tra operation intent;
  2. nếu đã có `destination_item_id`, verify;
  3. nếu thiếu response vì crash, search theo `appProperties` idempotency key;
  4. tìm được đúng một item → persist mapping và `Done`;
  5. không tìm thấy → đưa lại `Ready`;
  6. nhiều item → mark ambiguous, không tự tạo thêm.

## 6.3. Idempotency

Mỗi destination item do app tạo phải có private `appProperties` tối thiểu:

```json
{
  "gdclone_source_id": "<source id>",
  "gdclone_scope_id": "<job or watch id>",
  "gdclone_copy_key": "<stable idempotency key>",
  "gdclone_generation": "1"
}
```

Không dùng file name làm idempotency key.

`idempotency_key` phải deterministic theo:

```text
scope_type + scope_id + source_item_id + destination_parent_id + generation
```

External API và SQLite không có distributed transaction. Thiết kế phải chịu được crash tại mọi điểm:

1. sau khi ghi intent nhưng trước API call;
2. API thành công nhưng chưa nhận response;
3. đã nhận response nhưng chưa commit DB;
4. commit mapping xong nhưng cleanup item cũ chưa chạy.

---

# 7. Google OAuth đúng cho local-first

## 7.1. Luồng chính

Không dùng `/connect` làm nơi hoàn tất OAuth từ điện thoại.

Command chuẩn:

```bash
gdclone-bot auth login
gdclone-bot auth status
gdclone-bot auth revoke
```

`auth login`:

1. Bind listener vào `127.0.0.1` trên random free port trong range cấu hình.
2. Tạo cryptographically random `state`.
3. Tạo PKCE SHA-256 challenge/verifier.
4. Tạo auth URL với:
   - `response_type=code`;
   - `access_type=offline`;
   - `include_granted_scopes=true`;
   - `prompt=consent` chỉ khi cần xin refresh token mới;
   - loopback redirect.
5. Mở browser trên cùng máy nếu có desktop session; nếu không thì in URL.
6. Không follow redirect trong OAuth token HTTP client.
7. Validate `state`.
8. Exchange code bằng PKCE verifier.
9. Lưu refresh token an toàn.
10. Gọi Drive API `about.get` hoặc một request tối thiểu để xác nhận credential.
11. Listener trả trang HTML “Authorization completed; you may close this tab”.
12. Listener tự đóng.

`/connect` trên Telegram chỉ:

- hiển thị trạng thái account;
- hướng dẫn chạy `gdclone-bot auth login`;
- không tạo flow mà user mở trên thiết bị khác rồi redirect về localhost của thiết bị đó.

## 7.2. Scope

Để clone arbitrary Drive links mà user có quyền truy cập, `drive.file` thường không đủ. Mặc định dùng:

```text
https://www.googleapis.com/auth/drive
```

Phải giải thích rõ đây là broad/restricted scope và chỉ dùng cho local personal app.

Docs phải cảnh báo:

- OAuth consent screen `External + Testing` có thể phát refresh token chỉ sống 7 ngày với scope ngoài basic profile;
- app phải xử lý `invalid_grant` và yêu cầu reconnect;
- không tạo refresh token mới mỗi lần startup;
- revoke token khi user gọi `auth revoke`.

---

# 8. Drive link parser và resource key

Hỗ trợ:

```text
https://drive.google.com/file/d/{id}/view
https://drive.google.com/drive/folders/{id}
https://drive.google.com/open?id={id}
https://docs.google.com/document/d/{id}
https://docs.google.com/spreadsheets/d/{id}
https://docs.google.com/presentation/d/{id}
raw Drive ID khi dùng /clone
resourcekey / resourceKey query parameter
```

Parser trả:

```rust
struct DriveReference {
    file_id: String,
    resource_key: Option<String>,
    hinted_kind: Option<DriveItemKind>,
}
```

Không xác định type chỉ từ URL. Luôn gọi `files.get`.

Khi có resource key, Drive client thêm:

```text
X-Goog-Drive-Resource-Keys: fileId/resourceKey
```

Resource key của shortcut target lấy từ `shortcutDetails.targetResourceKey`.

Validation:

- giới hạn length;
- chỉ nhận host Google Drive/Docs đã biết;
- raw ID chỉ cho phép charset hợp lệ;
- không fetch URL người dùng đưa;
- không follow URL redirect tùy ý;
- không log resource key.

---

# 9. Drive API client

## 9.1. Fields tối thiểu

`files.get`/`files.list` phải yêu cầu rõ `fields`, không dùng wildcard bừa:

```text
id,
name,
mimeType,
size,
parents,
driveId,
resourceKey,
shortcutDetails(targetId,targetMimeType,targetResourceKey),
trashed,
modifiedTime,
md5Checksum,
version,
capabilities(canCopy,canAddChildren,canListChildren,canRename,
             canMoveItemWithinDrive,canTrash),
copyRequiresWriterPermission,
appProperties
```

Không giả định:

- file Shared Drive có owner;
- mọi file có `size`;
- Google-native file có MD5;
- mọi item có thể copy;
- HTTP 404 luôn là “không tồn tại” — cũng có thể là không có quyền.

## 9.2. Methods

Implement typed wrappers cho:

- `about.get`;
- `files.get`;
- `files.list`;
- `files.create`;
- `files.copy`;
- `files.update`;
- `files.delete` chỉ khi policy yêu cầu và có confirmation;
- `changes.getStartPageToken`;
- `changes.list`;
- token refresh/revoke.

Shared Drive:

- đặt `supportsAllDrives=true` cho method liên quan;
- `files.list` dùng `includeItemsFromAllDrives=true`;
- khi search một Shared Drive: `corpora=drive&driveId=...`;
- `changes.list` Shared Drive dùng `driveId`, `includeItemsFromAllDrives=true`, `supportsAllDrives=true`;
- cursor scope theo từng shared drive.

## 9.3. Folder

Google Drive không copy folder bằng `files.copy`.

Clone folder phải:

1. `files.create` destination folder;
2. persist mapping;
3. list source children;
4. create child folders;
5. copy non-folder items.

## 9.4. Shortcut policy

Mặc định: `preserve`.

Policy:

### `preserve`

Tạo shortcut mới trỏ tới target gốc nếu user còn quyền.

### `remap_internal`

- nếu target nằm trong source tree và đã có mapping → shortcut mới trỏ tới destination target;
- nếu target chưa được clone → defer shortcut operation;
- nếu target ngoài tree → preserve target gốc.

### `resolve`

Clone target content. Chỉ bật explicit vì:

- có thể kéo dữ liệu ngoài watched root;
- folder shortcut có thể tạo cycle;
- phải có visited set, cycle detection và max expansion depth.

Không mặc định “resolve mọi shortcut”.

## 9.5. Google-native files

Dùng `files.copy` khi `capabilities.canCopy=true`.

Không tuyên bố mọi Google MIME type đều copy được. Unsupported/capability-denied item:

- mark `skipped_unsupported` hoặc `failed_permission`;
- ghi rõ trong report;
- report phải phân loại lỗi quyền/quota/rate-limit/resource-key rõ ràng;
- không tự gửi request xin quyền hoặc hướng người dùng bấm xin quyền trong bot;
- không download/export/import ngầm.

---

# 10. One-shot clone engine

## 10.1. Discovery pipeline

Không recursion bằng call stack.

Dùng durable BFS queue trong `traversal_folders`.

Mỗi folder:

1. claim row `pending -> listing`;
2. call `files.list` với `pageSize` hợp lý;
3. transaction:
   - insert child folders/items bằng unique constraints;
   - persist `next_page_token`;
   - update count;
4. nếu còn page token → tiếp tục;
5. hết page → `done`.

Có thể copy file song song khi destination parent mapping đã sẵn sàng; không bắt buộc đợi scan toàn cây.

Progress hiển thị:

- discovered count tăng dần;
- total là “unknown” cho đến khi discovery hoàn tất;
- không hiển thị ETA giả khi tổng chưa biết.

## 10.2. Scheduler

Dùng bounded work queue.

Giới hạn riêng:

- global Drive write concurrency;
- per-account concurrency;
- list/read concurrency;
- Telegram edit concurrency.

Default ban đầu:

```text
copy/create concurrency: 5
list concurrency: 2
active clone jobs: 2
active job per Telegram user: 1
```

Giới hạn là configurable, không hardcode.

## 10.3. Duplicate policies

### `keep_both`

Luôn tạo copy mới, nhưng resume cùng operation không được tạo duplicate.

### `skip_same_source`

Skip nếu destination parent đã có item do app tạo với cùng:

- `gdclone_source_id`;
- scope compatible;
- canonical destination parent.

### `replace_safe`

1. Tạo copy mới trước.
2. Verify response và persist new mapping.
3. Mark old destination `cleanup_pending`.
4. Chỉ trash old item sau khi mapping mới durable.
5. Nếu crash, recovery hoàn tất cleanup.
6. Không delete vĩnh viễn.
7. Destination ID thay đổi và phải ghi trong report.

Không match duplicate chỉ bằng name.

---

# 11. Retry, backoff và adaptive pacer

## 11.1. Error classification

Phân loại theo HTTP status **và Google error reason**.

Retryable:

- network timeout/reset;
- 429;
- 500, 502, 503, 504;
- 403 với reason như `rateLimitExceeded`, `userRateLimitExceeded`;
- một số backend/transient error đã được allowlist.

Refresh flow:

- 401 hoặc token expiry → singleflight refresh;
- retry request đúng một lần sau refresh;
- `invalid_grant` → account `reconnect_required`, không loop vô hạn.

Permanent hoặc cần user action:

- malformed request;
- invalid ID;
- `insufficientPermissions`;
- `notFound` sau khi đã kiểm tra Shared Drive params/resource key;
- unsupported operation;
- destination không cho add children;
- copy restriction;
- storage/quota condition không tự hết sớm.

## 11.2. Backoff

```text
delay = min(base * 2^attempt, max_delay) + full_jitter
```

- default base: 1s;
- max: 64s;
- max attempts theo operation;
- honor `Retry-After` nếu response có;
- log reason/code, không log token hoặc file name nhạy cảm.

## 11.3. Shared adaptive controller

Không chỉ retry riêng từng item, vì nhiều worker có thể cùng gây rate limit.

Mỗi Google account có `DrivePacer` dùng:

- semaphore;
- minimum request spacing khi cần;
- AIMD-like concurrency adjustment:
  - success window ổn định → tăng chậm;
  - 429/rate-limit → giảm nhanh;
- cooldown chung cho account;
- circuit breaker ngắn khi lỗi 5xx dày đặc.

Một item lỗi không đóng băng toàn hệ thống vĩnh viễn, nhưng rate-limit signal phải ảnh hưởng đến toàn account để tránh thundering herd.

---

# 12. Watch/Sync architecture

## 12.1. Tách subscription, cursor và job

- `watch_subscriptions`: cấu hình dài hạn.
- `change_cursors`: một poller cho account + corpus.
- `change_events`: raw durable event log.
- `watch_event_applications`: trạng thái áp event cho từng watch.
- `jobs`: finite work để clone/apply/reconcile.

Không dùng một `jobs.mode=watch` chạy vô hạn.

## 12.2. Một poller cho mỗi corpus

Key:

```text
google_account_id + corpus_kind + drive_id
```

- My Drive/user corpus: một cursor.
- Mỗi Shared Drive: cursor riêng.

Poller gọi `changes.list`, không rescan toàn bộ watched tree mỗi chu kỳ.

## 12.3. Commit cursor đúng

Cho page token `P`:

1. Fetch page ngoài transaction.
2. Begin SQLite transaction.
3. Insert toàn bộ raw `change_events` của page với:
   - `request_page_token=P`;
   - `ordinal_in_page`;
   - payload hiện tại hoặc removed marker.
4. Update cursor:
   - `nextPageToken` nếu còn page;
   - `newStartPageToken` nếu đã tới cuối.
5. Commit.

Nếu crash trước commit, page được fetch lại và unique constraint dedupe.

Nếu crash sau commit, cursor đã advance cùng transaction.

`newStartPageToken` chỉ dùng khi tới cuối change list.

## 12.4. Race-free watch creation

Không clone trước rồi mới bắt đầu theo dõi.

Quy trình:

1. Đảm bảo corpus poller đang chạy và cursor đã có.
2. Trong một transaction:
   - đọc `last_event_sequence`;
   - tạo subscription `initializing`;
   - lưu `baseline_sequence`.
3. Chạy initial clone và xây mapping.
4. Poller tiếp tục ghi raw changes trong lúc clone.
5. Sau initial clone:
   - chuyển `catching_up`;
   - replay mọi `change_events.sequence > baseline_sequence`;
   - reconcile mapping/tree.
6. Khi catch-up tới current cursor sequence:
   - set `last_consumed_sequence`;
   - chuyển `active`.
7. Không có khoảng hở giữa initial clone và watch.

Raw event retention chỉ được prune khi mọi subscription còn hoạt động đã consume qua sequence đó, hoặc sau khi watch bị buộc `needs_reconcile` vì vượt retention/backlog limit.

## 12.5. Pause semantics

`/watch_pause` chỉ dừng **apply** cho subscription.

Corpus poller vẫn chạy và raw events vẫn được lưu, vì cursor được dùng chung cho nhiều watch.

Paused subscription:

- giữ `last_consumed_sequence`;
- backlog tăng;
- nếu backlog vượt giới hạn cấu hình → `needs_reconcile`;
- resume sẽ replay backlog hoặc full reconcile.

Không pause shared cursor chỉ vì một watch pause.

## 12.6. Membership logic

Changes API trả thay đổi của corpus, không riêng watched folder.

Một event thuộc watch khi:

- source ID đã có mapping trong watch; hoặc
- parent hiện tại có mapping active; hoặc
- event là move của known item; hoặc
- reconcile chứng minh item nằm dưới watched root.

Rules:

### New item

Parent là mapped source folder → clone vào mapped destination parent.

### Rename

Known mapped item, `name` đổi → `files.update(name=...)` trên destination nếu capability cho phép.

### Move inside tree

Known item, parent mới là mapped folder → update destination parent theo mapping.

### Move outside tree

Known item, parent mới không thuộc watched tree:

- mặc định giữ destination;
- mapping state `detached`;
- không tiếp tục content sync;
- không delete destination.

### Move back

Source ID cũ xuất hiện dưới mapped parent:

- reactivate mapping;
- move existing destination theo;
- không tạo copy mới.

### Source removed/trashed/lost access

`removed=true` hoặc source không còn accessible:

- nếu source đã mapped → `source_removed`;
- giữ destination;
- notify một lần;
- không phát cảnh báo lặp;
- không thể luôn phân biệt delete và loss of access, nên UI dùng wording trung tính.

### Folder moved out

Mark cả mapped subtree detached bằng transaction/recursive CTE hoặc traversal an toàn.

## 12.7. Phân loại content change

Drive change feed cung cấp current state, không cung cấp semantic diff hoàn hảo.

Lưu fingerprint:

```text
name
parents
mimeType
trashed
version
modifiedTime
md5Checksum nullable
size nullable
```

Classifier:

1. So metadata fields để nhận rename/move/trash.
2. Binary:
   - MD5 thay đổi → content changed;
   - nếu MD5 thiếu, dùng version + modifiedTime một cách bảo thủ.
3. Google-native:
   - không dựa vào MD5;
   - version/modifiedTime thay đổi sau khi loại trừ metadata-only diff → possible content change.
4. Ambiguous → policy bảo thủ hoặc manual confirmation.

Không tuyên bố có thể xác định 100% content-only change cho mọi Google-native file chỉ từ Drive metadata.

## 12.8. Content update policies

### `versioned_copy` — mặc định an toàn

1. Copy version mới.
2. New copy trở thành active mapping.
3. Old destination được move vào history folder do app quản lý hoặc giữ với tên timestamp.
4. Không delete dữ liệu.
5. Report old/new ID.

### `replace_copy`

1. Copy mới trước.
2. Verify và persist active mapping.
3. Trash old destination.
4. Không permanent delete.
5. Destination ID thay đổi.

### `manual_confirmation`

Gửi Telegram notification và chờ user chọn.

`files.copy` không overwrite content của destination file cũ. Không được mô tả “update in place” nếu thực tế là tạo file mới.

## 12.9. Adaptive polling

Default:

```text
recent changes / backlog active: 20s
idle >= 10 minutes: 60s
idle >= 1 hour: 300s
transient error: exponential backoff + jitter
```

Không dùng lịch ngày/đêm cố định.

Poll interval là account/corpus-level, không per-watch.

---

# 13. Telegram UX

## 13.1. Commands

```text
/start
/help
/account
/clone <url-or-id>
/jobs
/status <job_id>
/pause <job_id>
/resume <job_id>
/cancel <job_id>
/retry <job_id>

/watch <url-or-id>
/watches
/watch_status <watch_id>
/watch_pause <watch_id>
/watch_resume <watch_id>
/watch_policy <watch_id>
/unwatch <watch_id>

/whoami
/grant <telegram_user_id>
/revoke <telegram_user_id>
```

`/connect` nếu giữ lại chỉ alias tới hướng dẫn local auth, không mở OAuth giả từ xa.

## 13.2. Clone flow

```text
User: paste Drive URL
Bot:
  - parse ID + resource key
  - files.get
  - validate capabilities
  - show source metadata
  - ask mode: [Clone một lần] [Clone + Watch]
  - ask destination
  - ask duplicate/shortcut/update policy
  - confirm
  - create durable job
```

Metadata hiển thị:

- name;
- file/folder;
- MIME category;
- known size;
- My Drive/Shared Drive;
- canCopy/canListChildren;
- warning nếu resource key, restriction hoặc unsupported type.

Không hiển thị owner nếu API không trả.

## Automatic destination root creation

The normal clone flow must not require the user to manually create a
destination folder matching the source folder.

A configured destination represents the destination parent, not the
final clone root.

For a source folder named:

    Course Materials

and a configured destination parent:

    My Drive/Telegram Clone

the bot must automatically create:

    My Drive/Telegram Clone/Course Materials

and clone the complete source tree inside that newly created folder.

The destination root folder name must default to the exact source folder
name returned by Google Drive API.

The user must not be required to:

- create the destination root folder manually;
- paste a destination link for every clone;
- browse the destination tree for every clone.

### Default destination

Support a persistent default destination per Google account.

Add Telegram commands:

- /set_destination <folder_url_or_id>
- /destination
- /clear_destination

`/set_destination` must:

1. Parse the folder URL and resource key.
2. Call `files.get`.
3. Confirm that the item is a folder.
4. Confirm that the authenticated Google account can add children.
5. Persist:
   - destination parent ID;
   - destination Drive ID;
   - resource key if required;
   - display name;
   - validation timestamp.

When a Drive URL is pasted:

1. Inspect the source.
2. Resolve the configured default destination.
3. Validate the destination capability.
4. Show a confirmation message:

   Source: Course Materials
   Destination: My Drive/Telegram Clone/Course Materials

   [Clone now] [Change destination] [Cancel]

The user may change the destination, but this must be optional.

### Fast clone mode

Support:

- /clone <url>
- /clone_here <url>
- direct Drive URL paste

When `auto_confirm_clone=false`, show one confirmation message.

When `auto_confirm_clone=true`, immediately create the durable clone job
using the configured destination.

Never enable auto-confirm by default until a valid destination has been
configured.

### Root folder creation

For a source folder:

1. Create a durable `create_folder` operation intent.
2. Generate a deterministic idempotency key from:
   - source folder ID;
   - destination parent ID;
   - job or watch scope ID;
   - generation.
3. Call `files.create` with:
   - source folder name;
   - folder MIME type;
   - destination parent ID;
   - `appProperties` containing the source ID and idempotency key.
4. Persist the source-root to destination-root mapping immediately.
5. Only then begin child traversal.

If the application crashes after Google creates the folder but before
SQLite commits the destination ID, recovery must search by the private
`appProperties` idempotency key and reuse the existing folder.

It must not create another root folder blindly.

### Folder naming

By default, preserve the source folder name exactly.

Do not treat `/` in a Google Drive name as a filesystem path separator.
Google Drive item names are metadata, not local filesystem paths.

Only remove invalid control characters required by the Telegram display
layer; do not unnecessarily rewrite the Drive item name.

Allow an optional custom root name before confirmation.

### Existing destination folders

Google Drive allows multiple sibling items with the same name.
Therefore name equality alone must never be used to determine whether a
clone already exists.

Use this policy:

1. If an existing folder has matching application idempotency metadata:
   - reuse it;
   - continue or resume the existing clone.

2. If a folder only has the same visible name but no matching metadata:
   - `keep_both`: create another folder with the same source name;
   - `reuse_selected`: only reuse after explicit user confirmation;
   - `append_suffix`: create `Course Materials (2)` or a configured
     timestamp suffix.

Default to `keep_both` for a new independent clone and
`reuse matching idempotency metadata` for recovery.

Never merge into an unrelated same-name folder automatically.

### Source file behavior

For a single source file, copy it directly into the configured
destination parent by default.

Optionally support:

    wrap_single_file_in_folder=true

When enabled, create a wrapper folder named from the source file before
copying it.

### Watch mode

When creating a watch subscription:

1. Automatically create the destination root folder using the source
   root folder name.
2. Persist the root mapping.
3. Perform the race-free initial clone.
4. Apply subsequent changes inside that mapped root.

If the watched source root itself is renamed, rename the mapped
destination root unless the user has explicitly locked its custom name.

## 13.3. Destination browser

Inline folder browser:

- My Drive root;
- Shared Drives;
- pagination;
- parent navigation;
- recent destinations;
- paste destination URL;
- create folder nếu capability cho phép.

Telegram `callback_data` chỉ chứa short opaque state ID. State lưu server-side và gắn:

- telegram user ID;
- chat ID;
- expiry;
- action type.

Không nhét Drive token, file ID dài hoặc JSON lớn trực tiếp vào callback.

## 13.4. Progress actor

Mỗi job có một progress actor:

- coalesce updates;
- tối đa một edit trong interval cấu hình;
- xử lý Telegram `retry_after`;
- không để worker tự edit message trực tiếp;
- restart có thể restore hoặc tạo message mới.

Nội dung:

```text
State: Running
Discovered: 3,420
Completed: 2,105
Failed: 3
Skipped: 17
In flight: 5
Rate: 4.2 ops/s
Elapsed: 08:14
ETA: unavailable / estimated only when meaningful
```

Không spam message.

## 13.5. Authorization

Mặc định chỉ owner.

`operator` nếu được grant:

- dùng cùng Google account đã cấu hình của installation;
- không tự có OAuth riêng ở v1;
- mọi action audit theo Telegram user;
- không được grant/revoke user hoặc đổi security config.

Nếu muốn per-user Google account, đó là v2 và phải thiết kế OAuth remote phù hợp; không giả ghép loopback desktop flow vào nhiều user từ điện thoại.

---

# 14. Config

```toml
[telegram]
bot_token = "REPLACE_ME"
owner_telegram_id = 0
progress_edit_min_interval_ms = 3000

[destination]
auto_use_default = true
auto_confirm_clone = false
wrap_single_file_in_folder = false
root_name_policy = "preserve"
same_name_policy = "keep_both"

[google_oauth]
client_id = "REPLACE_ME.apps.googleusercontent.com"
client_secret = "REPLACE_ME"
redirect_port_start = 51000
redirect_port_end = 51100
scope = "https://www.googleapis.com/auth/drive"

[engine]
max_active_jobs = 2
max_active_jobs_per_user = 1
initial_write_concurrency = 5
min_write_concurrency = 1
max_write_concurrency = 8
list_concurrency = 2
max_retry_attempts = 6
retry_base_delay_ms = 1000
retry_max_delay_ms = 64000
request_timeout_seconds = 60
default_duplicate_policy = "skip_same_source"
default_shortcut_policy = "preserve"

[watch]
enabled = true
active_poll_seconds = 20
warm_idle_poll_seconds = 60
cold_idle_poll_seconds = 300
warm_idle_after_seconds = 600
cold_idle_after_seconds = 3600
max_backlog_events_per_watch = 100000
raw_event_retention_days = 30
default_content_update_policy = "versioned_copy"
default_deletion_policy = "preserve_destination"
default_move_out_policy = "detach"

[storage]
db_path = ""
log_dir = ""
report_dir = ""
secret_backend = "auto"

[security]
redact_file_names_in_info_logs = true
report_retention_days = 30
allow_operators = false

[platform]
startup_mode = "manual"
```

Path rỗng nghĩa là resolve bằng `directories` crate:

- Linux: XDG data/config dirs;
- Windows: user application data dirs.

Env override prefix:

```text
GDCLONE__
```

Ví dụ:

```text
GDCLONE__TELEGRAM__BOT_TOKEN
GDCLONE__TELEGRAM__OWNER_TELEGRAM_ID
```

Startup validate toàn bộ config và fail fast với message cụ thể.

---

# 15. Security requirements

Bắt buộc:

- không log bot token, client secret, auth code, PKCE verifier, access token, refresh token, resource key;
- token redaction ở error/debug formatting;
- config file permission check;
- DB/key/report directory chỉ current user truy cập;
- loopback listener bind `127.0.0.1`, không `0.0.0.0`;
- validate OAuth state;
- PKCE;
- OAuth HTTP client không follow redirect;
- refresh singleflight;
- revoke support;
- allowlist ở middleware trước mọi command/callback;
- callback ownership + expiry;
- command rate limit;
- Drive URL parser không fetch arbitrary URL;
- SQL parameterized;
- Telegram Markdown/HTML escape;
- report chỉ gửi đúng chat khởi tạo job;
- audit admin action;
- không log full file path/name ở INFO nếu redaction bật;
- không auto-delete destination do source delete;
- destructive policy cần explicit confirmation;
- `master.key` không commit;
- `.env`/config secrets không commit;
- không chạy Windows Service dưới LocalSystem nếu credential thuộc CurrentUser;
- không khuyến khích tắt security/quota của Google.

Tạo:

- `SECURITY.md`;
- `docs/threat-model.md`;
- secret rotation procedure;
- OAuth reconnect procedure;
- backup/restore guide;
- report privacy warning.

---

# 16. CachyOS deployment

## 16.1. Setup

```bash
sudo pacman -Syu
sudo pacman -S --needed rustup git
rustup default stable

cargo build --release
install -Dm755 target/release/gdclone-bot \
  "$HOME/.local/bin/gdclone-bot"

gdclone-bot auth login
gdclone-bot doctor
gdclone-bot run
```

SQLite được bundled, không bắt buộc cài SQLite runtime; `sqlite` CLI chỉ là tool hỗ trợ debug.

## 16.2. systemd user unit

```ini
[Unit]
Description=gdclone-bot
Wants=network-online.target
After=network-online.target

[Service]
Type=simple
ExecStart=%h/.local/bin/gdclone-bot run
Restart=on-failure
RestartSec=5
TimeoutStopSec=30
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=%h/.local/share/gdclone-bot
ReadWritePaths=%h/.config/gdclone-bot

[Install]
WantedBy=default.target
```

Agent phải kiểm tra hardening directive tương thích với đường dẫn thật, không copy mù.

Cài:

```bash
mkdir -p ~/.config/systemd/user
cp packaging/gdclone-bot.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now gdclone-bot.service
journalctl --user -u gdclone-bot -f
```

Để user service chạy từ boot và tồn tại sau logout:

```bash
loginctl enable-linger "$USER"
```

Phải giải thích đây là lựa chọn, không tự bật ngầm.

Auth phải chạy tương tác **trước** khi service headless bắt đầu.

---

# 17. Windows 10/11 deployment

## 17.1. Build và chạy tương tác

```powershell
cargo build --release
.\target\release\gdclone-bot.exe auth login
.\target\release\gdclone-bot.exe doctor
.\target\release\gdclone-bot.exe run
```

## 17.2. Startup mặc định

Dùng Task Scheduler “At logon” dưới chính user đã chạy OAuth.

Script:

```text
scripts/install-windows-task.ps1
```

Phải:

- dùng absolute executable path;
- dùng user profile đúng;
- set restart on failure;
- không lưu secret trong task arguments;
- ghi log vào application data directory.

## 17.3. Windows Service — advanced

Có thể dùng `windows-service`, nhưng:

- service phải chạy dưới đúng user account có credential;
- không mặc định LocalSystem;
- implementation phải handle Stop/Interrogate;
- report `StartPending`, `Running`, `StopPending`, `Stopped`;
- graceful shutdown chờ worker checkpoint;
- nếu secret backend không accessible trong service context thì fail rõ ràng.

Không cross-build binary MSVC thật trên CachyOS rồi tuyên bố thành công. Dùng CI matrix:

```text
ubuntu-latest
windows-latest
```

CachyOS có thể chạy `cargo check`; Windows release build và service verification phải chạy trên Windows thật.

---

# 18. Observability và maintenance

Logging fields:

```text
correlation_id
job_id
watch_id
operation_id
google_account_id
http_status
google_error_reason
retry_attempt
```

Không log source/destination name ở INFO khi privacy mode bật.

Health qua CLI:

```bash
gdclone-bot doctor
gdclone-bot status
```

`doctor` kiểm tra:

- config;
- writable paths;
- DB migration/integrity;
- secret store;
- Google token refresh;
- Drive API access;
- Telegram getMe;
- webhook đang bị set hay không;
- network;
- system clock;
- pending recovery.

Backup:

- pause writes hoặc dùng SQLite backup API;
- backup DB + encrypted credential blob + key theo hướng dẫn riêng;
- không copy DB đang WAL-active bằng cách ngây thơ nếu chưa checkpoint/backup API.

---

# 19. Test strategy

## 19.1. Unit tests

- Drive URL/ID parser;
- resource key parser/header;
- OAuth state/PKCE;
- error classification;
- backoff bounds/jitter;
- state transitions;
- idempotency key;
- duplicate policy;
- shortcut cycle detection;
- watch membership;
- fingerprint classifier;
- Telegram escaping;
- config merge/env override.

## 19.2. Integration tests với mock HTTP server

- paginated `files.list`;
- Shared Drive params;
- token refresh singleflight;
- 429/403 rate limit;
- 5xx;
- response success rồi simulated crash trước DB commit;
- reconciliation qua `appProperties`;
- Telegram retry_after;
- change page replay/dedupe.

## 19.3. SQLite crash/recovery tests

- kill process khi item `copying`;
- kill sau Drive success trước mapping commit;
- kill giữa change page commit;
- DB WAL recovery;
- duplicate prevention;
- migration rollback.

## 19.4. Manual end-to-end acceptance

### One-shot

1. Clone một binary file.
2. Clone folder 3+ levels.
3. Clone empty folder.
4. Clone Docs/Sheets/Slides.
5. Clone My Drive → My Drive.
6. Shared Drive → My Drive.
7. My Drive → Shared Drive.
8. Shared Drive → Shared Drive.
9. Link cần resource key.
10. Shortcut nội bộ.
11. Shortcut ngoài tree.
12. Cancel giữa job.
13. Restart giữa job.
14. Rate-limit/transient retry.
15. Permission denied.
16. Destination không có `canAddChildren`.
17. Không tạo duplicate sau replay.

### Watch

1. File tạo trong initial clone window không bị bỏ sót.
2. Hai watch cùng corpus dùng một cursor.
3. Rename.
4. Move trong tree.
5. Move ra ngoài.
6. Move trở lại.
7. Source trashed.
8. Source mất quyền.
9. Binary content update.
10. Google Docs update.
11. Watch pause vẫn giữ backlog.
12. Resume replay backlog.
13. Restart không mất cursor.
14. Crash trước cursor commit không mất event.
15. Replay cùng page không duplicate.
16. Shared Drive cursor riêng.
17. Replace policy crash giữa copy và cleanup.
18. Event log retention không prune event chưa consume.

---

# 20. Roadmap triển khai

## Phase 0 — Research và ADR

- clone 6 repo;
- xác nhận license + commit SHA;
- đọc official docs;
- tạo `reference-analysis.md`;
- tạo ADR cho:
  - OAuth local CLI;
  - SQLite actor;
  - idempotency;
  - shortcut policy;
  - watch event log;
  - Windows startup.

Không code core trước khi xong phase này.

## Phase 1 — Foundation

- Cargo project;
- config + directories;
- tracing/redaction;
- SQLite actor + migrations;
- secret store;
- CLI;
- OAuth login/status/revoke;
- Drive client get/list basic;
- `/start`, `/account`;
- unit tests.

Chạy thật trên CachyOS.

## Phase 2 — One-shot durable clone

- link/resource key parser;
- source inspection;
- destination validation;
- discovery BFS;
- folder create;
- files.copy;
- mappings;
- operation intents;
- checkpoint;
- recovery;
- bounded scheduler;
- progress actor.

Kill-process test bắt buộc.

## Phase 3 — Reliability và UX

- **Phase 3.1 — Permission/quota diagnostics**
  - Drive error translator cho Telegram;
  - report `error_category`;
  - báo “không dùng được” với file/folder bị policy/quyền chặn;
  - không xin quyền hộ user, không bypass/capture.
- **Phase 3.2 — Operational controls**
  - `/status`, `/pause`, `/resume`, `/cancel`, `/retry` nhận full ID, short ID,
    và mặc định job active mới nhất khi hợp lý;
  - `/last_report`;
  - progress %, elapsed, rate, ETA;
  - callback hết hạn không làm lỗi handler.
- **Phase 3.3 — Shared Drive/resource-key polish**
  - test My Drive ↔ Shared Drive;
  - thông báo rõ thiếu resource key/quyền;
  - report lỗi Shared Drive dễ hiểu.
  - trạng thái hiện tại: client đã gửi `supportsAllDrives`,
    `includeItemsFromAllDrives` và resource-key header ở luồng chính; Telegram
    confirmation đã hiển thị presence resource key và lý do quét trước thất bại,
    nhưng vẫn cần test thật với Shared Drive.
- **Phase 3.4 — Destination UX**
  - recent destinations;
  - optional folder browser nếu paste link gây khó chịu;
  - không làm dashboard web.
  - trạng thái hiện tại: recent destinations đã có; browser My Drive tối giản
    đã có, giới hạn 20 folder đầu/trang đầu; pagination và Shared Drive root
    listing để sau khi có nhu cầu thật.
- **Phase 3.5 — One-shot hardening**
  - adaptive pacer/retry classification;
  - duplicate/shortcut policies vừa đủ;
  - JSON/CSV report;
  - systemd packaging;
  - security hardening.

One-shot acceptance phải xanh trước Watch.

## Phase 4 — Watch ingestion

- `change_cursors`;
- một poller mỗi corpus;
- raw `change_events`;
- atomic page commit;
- retention;
- adaptive poll;
- watch create + race-free initial clone/catch-up.

Chưa apply destructive/content update ở đầu phase.

## Phase 5 — Watch apply/reconcile

- membership;
- rename/move;
- detach/reconnect;
- removed/lost access;
- fingerprint classifier;
- versioned/replace/manual content policies;
- pause backlog;
- full reconciliation;
- Telegram watch commands.

Chạy toàn bộ Watch acceptance.

## Phase 6 — Windows

- Windows paths;
- DPAPI/key context;
- Task Scheduler script;
- optional Windows Service;
- OAuth loopback;
- one-shot flow;
- watch flow;
- restart/recovery;
- CI Windows build.

Không gọi Windows “supported” trước khi test thật.

## Phase 7 — Final hardening

- threat model;
- backup/restore;
- docs;
- performance profiling;
- slow query review;
- error message quality;
- dependency audit;
- license audit;
- final acceptance matrix.

---

# 21. Definition of Done

Project chỉ được coi hoàn thành khi:

- `cargo fmt --check` xanh;
- `cargo clippy --all-targets --all-features -- -D warnings` xanh;
- `cargo test --all-features` xanh;
- release build CachyOS xanh;
- Windows CI build xanh;
- migration từ empty DB chạy được;
- restart recovery test xanh;
- duplicate-after-crash test xanh;
- one-shot acceptance core xanh;
- watch cursor replay test xanh;
- không có secret trong Git history/current tree;
- docs setup chạy được theo đúng command;
- không còn TODO trong core paths;
- final report ghi trung thực test nào đã/chưa chạy.

---

# 22. Final report format cho agent

```markdown
## Đã triển khai
## Quyết định kiến trúc
## Thay đổi so với spec ban đầu
## Các file chính
## Database và migration
## OAuth và secret storage
## Clone engine
## Recovery và idempotency
## Watch/Sync
## Cách chạy trên CachyOS
## Cách chạy trên Windows
## Test đã chạy
## Lỗi đã sửa
## Hạn chế còn lại
## Rủi ro vận hành
## Bước tiếp theo
```

Không nói “production-ready” nếu chưa chạy acceptance thật.

---

# 23. Official references phải được agent đối chiếu

- Google Drive API — Retrieve changes:  
  https://developers.google.com/workspace/drive/api/guides/manage-changes
- Google Drive API — `changes.list`:  
  https://developers.google.com/workspace/drive/api/reference/rest/v3/changes/list
- Google Drive API — Shared Drive support:  
  https://developers.google.com/workspace/drive/api/guides/enable-shareddrives
- Google Drive API — `files.copy`:  
  https://developers.google.com/workspace/drive/api/reference/rest/v3/files/copy
- Google Drive API — File resource/capabilities/appProperties/version:  
  https://developers.google.com/workspace/drive/api/reference/rest/v3/files
- Google Drive API — Resource keys:  
  https://developers.google.com/workspace/drive/api/guides/resource-keys
- Google Drive API — Custom properties:  
  https://developers.google.com/workspace/drive/api/guides/properties
- Google Drive API — Shortcuts:  
  https://developers.google.com/workspace/drive/api/guides/shortcuts
- Google Drive API — Error handling:  
  https://developers.google.com/workspace/drive/api/guides/handle-errors
- Google OAuth desktop/native apps:  
  https://developers.google.com/identity/protocols/oauth2/native-app
- Google OAuth refresh-token expiration:  
  https://developers.google.com/identity/protocols/oauth2
- Telegram Bot API:  
  https://core.telegram.org/bots/api
- `oauth2` crate:  
  https://docs.rs/oauth2/
- `tokio-rusqlite` crate:  
  https://docs.rs/tokio-rusqlite/
- `windows-service` crate:  
  https://docs.rs/windows-service/
- systemd `loginctl enable-linger`:  
  https://www.freedesktop.org/software/systemd/man/latest/loginctl.html

Agent phải kiểm tra tài liệu tại thời điểm triển khai vì API/crate có thể thay đổi.
