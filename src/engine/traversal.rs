use std::collections::VecDeque;

use crate::drive::{client::DriveClient, types::DriveFile};

pub async fn breadth_first_list(
    client: &DriveClient,
    access_token: &str,
    root_folder_id: &str,
) -> anyhow::Result<Vec<DriveFile>> {
    let mut out = Vec::new();
    let mut queue = VecDeque::from([root_folder_id.to_string()]);

    while let Some(parent) = queue.pop_front() {
        let mut page_token = None;
        loop {
            let page = client
                .list_children(access_token, &parent, None, page_token.as_deref())
                .await?;
            for file in page.files {
                if file.is_folder() {
                    queue.push_back(file.id.clone());
                }
                out.push(file);
            }
            match page.next_page_token {
                Some(next) => page_token = Some(next),
                None => break,
            }
        }
    }

    Ok(out)
}
