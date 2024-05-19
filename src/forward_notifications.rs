use anyhow::{anyhow, Error};
use cln_plugin::Plugin;
use log::info;
use serde_json::Value;
use tokio::join;

use crate::{PluginState, OPT_LOG_FAILED_FORWARDS, OPT_LOG_SUCCESSFUL_FORWARDS};

struct ChannelInfo<'a> {
    short_id: &'a str,
    peer_alias: String,
}

pub async fn forward_event(plugin: Plugin<PluginState>, v: Value) -> Result<(), Error> {
    let event = match v.get("forward_event") {
        Some(p) => p,
        None => return Err(anyhow!("could not parse forward event")),
    };

    let status = match event.get("status") {
        Some(s) => s.as_str().unwrap(),
        None => {
            return Err(anyhow!("could not parse forward event status"));
        }
    };

    match status {
        "settled" => {
            if plugin.option(&OPT_LOG_SUCCESSFUL_FORWARDS)? {
                return log_forward_event_success(plugin, event).await;
            }

            Ok(())
        }
        "local_failed" => {
            if plugin.option(&OPT_LOG_FAILED_FORWARDS)? {
                return log_forward_event_failed(plugin, event).await;
            }

            Ok(())
        }
        _ => Ok(()),
    }
}

async fn log_forward_event_success(
    plugin: Plugin<PluginState>,
    event: &Value,
) -> Result<(), Error> {
    let (in_channel, out_channel) = get_channels_info(plugin, event).await?;
    info!(
        "Forwarded {}msat ({}msat fee) from {} ({}) to {} ({})",
        event.get("in_msat").unwrap(),
        event.get("fee_msat").unwrap(),
        in_channel.short_id,
        in_channel.peer_alias,
        out_channel.short_id,
        out_channel.peer_alias,
    );
    Ok(())
}

async fn log_forward_event_failed(plugin: Plugin<PluginState>, event: &Value) -> Result<(), Error> {
    let (in_channel, out_channel) = get_channels_info(plugin, event).await?;
    info!(
        "Forward for {}msat from {} ({}) to {} ({}) failed because: {}",
        event.get("in_msat").unwrap(),
        in_channel.short_id,
        in_channel.peer_alias,
        out_channel.short_id,
        out_channel.peer_alias,
        event.get("failreason").unwrap().as_str().unwrap()
    );
    Ok(())
}

async fn get_channels_info(
    plugin: Plugin<PluginState>,
    event: &Value,
) -> Result<(ChannelInfo, ChannelInfo), Error> {
    let in_channel = event.get("in_channel").unwrap().as_str().unwrap();
    let out_channel = event.get("out_channel").unwrap().as_str().unwrap();

    let (in_channel_info, out_channel_info) = join!(
        plugin
            .state()
            .clone()
            .node_names
            .get_channel_peer_alias(in_channel),
        plugin
            .state()
            .clone()
            .node_names
            .get_channel_peer_alias(out_channel)
    );

    Ok((
        parse_channel_info(in_channel, in_channel_info)?,
        parse_channel_info(out_channel, out_channel_info)?,
    ))
}

fn parse_channel_info(short_id: &str, info: Result<String, Error>) -> Result<ChannelInfo, Error> {
    Ok(ChannelInfo {
        short_id,
        peer_alias: info?,
    })
}
