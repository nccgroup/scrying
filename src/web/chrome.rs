use super::save;
use crate::{
    argparse::Opts, parsing::Target, reporting::ReportMessage, InputLists,
};
use chromiumoxide::cdp::browser_protocol::page::{
    CaptureScreenshotFormat, CaptureScreenshotParams,
};
use chromiumoxide::{Browser, BrowserConfig};
use color_eyre::{eyre::eyre, Result};
use futures::StreamExt;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};

pub async fn chrome_worker(
    targets: Arc<InputLists>,
    opts: Arc<Opts>,
    report_tx: mpsc::Sender<ReportMessage>,
    caught_ctrl_c: Arc<AtomicBool>,
) -> Result<()> {
    let (browser, mut handler) = Browser::launch(
        BrowserConfig::builder().build().map_err(|e| eyre!(e))?,
    )
    .await?;

    let _handle = tokio::task::spawn(async move {
        loop {
            let _event = handler.next().await.unwrap();
        }
    });

    for target in &targets.web_targets {
        if caught_ctrl_c.load(Ordering::SeqCst) {
            break;
        }

        // one day we will have let-else chains
        let u = if let Target::Url(target) = target {
            target
        } else {
            continue;
        };
        let page = browser.new_page(u.as_str()).await?;
        page.wait_for_navigation().await?;
        let params = CaptureScreenshotParams::builder()
            .format(CaptureScreenshotFormat::Png)
            .build();
        let img = page.screenshot(params).await?;
        save(target, &opts.output_dir, &img, &report_tx)?;
    }
    //handle.await?;
    Ok(())
}
