#!/usr/bin/env python3
import io
import json
import os
import sys
import time
import urllib.parse
import urllib.request
import zipfile
from pathlib import Path


def get_token():
    cred_path = Path.home() / ".git-credentials"
    if cred_path.exists():
        with open(cred_path) as f:
            for line in f:
                u = urllib.parse.urlparse(line.strip())
                if "github.com" in u.netloc and u.password:
                    return u.password
    token = os.environ.get("GITHUB_TOKEN")
    if token:
        return token
    raise RuntimeError("No GitHub token found in ~/.git-credentials or GITHUB_TOKEN")


def main():
    token = get_token()
    repo = "GiaHung07/Drive502"
    headers = {
        "Authorization": f"Bearer {token}",
        "User-Agent": "Drive502-Installer",
        "Accept": "application/vnd.github.v3+json",
    }

    print(f"Checking latest CI run in {repo}...")
    run_id = None
    while True:
        req = urllib.request.Request(
            f"https://api.github.com/repos/{repo}/actions/runs?per_page=1",
            headers=headers,
        )
        with urllib.request.urlopen(req) as resp:
            data = json.loads(resp.read().decode())
            runs = data.get("workflow_runs", [])
            if not runs:
                print("No workflow runs found. Retrying in 5s...")
                time.sleep(5)
                continue
            run = runs[0]
            run_id = run["id"]
            status = run["status"]
            conclusion = run["conclusion"]
            print(f"Run {run_id}: status={status}, conclusion={conclusion}")
            if status == "completed":
                if conclusion != "success":
                    print(f"Workflow finished with conclusion: {conclusion}")
                    sys.exit(1)
                break
        time.sleep(10)

    print("Fetching artifacts...")
    req = urllib.request.Request(
        f"https://api.github.com/repos/{repo}/actions/runs/{run_id}/artifacts",
        headers=headers,
    )
    with urllib.request.urlopen(req) as resp:
        data = json.loads(resp.read().decode())
        artifacts = data.get("artifacts", [])
        target_art = None
        for art in artifacts:
            if art["name"] == "gdclone-bot-linux-x86_64":
                target_art = art
                break
        if not target_art:
            print("Artifact gdclone-bot-linux-x86_64 not found in artifacts list!")
            sys.exit(1)

    download_url = target_art["archive_download_url"]
    print(f"Downloading artifact from {download_url}...")
    zip_path = "/tmp/gdclone-bot-download.zip"
    subprocess.run([
        "curl", "-fL",
        "-H", f"Authorization: Bearer {token}",
        "-H", "Accept: application/vnd.github.v3+json",
        download_url,
        "-o", zip_path
    ], check=True)

    dest_dir = Path.home() / ".local" / "bin"
    dest_dir.mkdir(parents=True, exist_ok=True)
    binary_dest = dest_dir / "gdclone-bot"

    with zipfile.ZipFile(zip_path) as zf:
        for name in zf.namelist():
            if name.endswith("gdclone-bot") or name == "gdclone-bot":
                with zf.open(name) as src, open(binary_dest, "wb") as dst:
                    dst.write(src.read())
                binary_dest.chmod(0o755)
                os.remove(zip_path)
                print(f"Successfully installed binary to {binary_dest}")
                return
    if os.path.exists(zip_path):
        os.remove(zip_path)


    print("Could not find gdclone-bot inside downloaded zip artifact.")
    sys.exit(1)


if __name__ == "__main__":
    main()
