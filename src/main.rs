use crate::logs::{forward_notifications, node_names::NodeNames};
use anyhow::{Error, anyhow};
use cln_plugin::Builder;
use cln_plugin::options::{ConfigOption, DefaultBooleanConfigOption};
use log::{error, info};
use std::path::Path;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

mod logs;

const OPT_LOG_FAILED_FORWARDS: DefaultBooleanConfigOption = ConfigOption::new_bool_with_default(
    "clog-forwards-failed",
    true,
    "Whether failed forwards should be logged",
);

const OPT_LOG_SUCCESSFUL_FORWARDS: DefaultBooleanConfigOption = ConfigOption::new_bool_with_default(
    "clog-forwards-successful",
    true,
    "Whether successful forwards should be logged",
);

#[derive(Clone, Debug)]
struct PluginState {
    pub node_names: NodeNames,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    unsafe {
        std::env::set_var(
            "CLN_PLUGIN_LOG",
            "cln_plugin=info,clog=debug,info,warn,error",
        )
    };

    let plugin = match Builder::new(tokio::io::stdin(), tokio::io::stdout())
        .dynamic()
        .option(OPT_LOG_FAILED_FORWARDS)
        .option(OPT_LOG_SUCCESSFUL_FORWARDS)
        .subscribe("forward_event", forward_notifications::forward_event)
        .configure()
        .await?
    {
        Some(p) => p,
        None => return Err(anyhow!("Could not configure plugin")),
    };

    let rpc_file =
        Path::new(&plugin.configuration().lightning_dir).join(plugin.configuration().rpc_file);

    let state = PluginState {
        node_names: match NodeNames::new(rpc_file).await {
            Ok(n) => n,
            Err(err) => {
                return Err(anyhow!("Could not initialize node name cache: {}", err));
            }
        },
    };

    let started_plugin = match plugin.start(state.clone()).await {
        Ok(p) => {
            info!(
                "Started plugin v{}-{}",
                built_info::PKG_VERSION,
                built_info::GIT_VERSION.unwrap_or("")
            );
            p
        }
        Err(e) => return Err(anyhow!("Could not start plugin: {}", e)),
    };

    started_plugin.join().await.unwrap_or_else(|e| {
        error!("Could not join plugin: {}", e);
    });

    Ok(())
}
