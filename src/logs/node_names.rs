use anyhow::{Error, anyhow};
use cln_rpc::ClnRpc;
use cln_rpc::model::requests::{GetinfoRequest, ListchannelsRequest, ListnodesRequest};
use cln_rpc::primitives::{PublicKey, ShortChannelId};
use moka::sync::{Cache, CacheBuilder};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

const CACHE_SIZE: u64 = 10_000;
const CACHE_EXPIRY: u64 = 60 * 60;

#[derive(Clone, Debug)]
pub struct NodeNames {
    rpc_file: PathBuf,
    own_pubkey: PublicKey,

    channel_peer_cache: Cache<ShortChannelId, PublicKey>,
    name_cache: Cache<PublicKey, String>,
}

impl NodeNames {
    pub async fn new(rpc_file: PathBuf) -> Result<Self, Error> {
        let mut rpc = ClnRpc::new(rpc_file.clone()).await?;
        let info_res = rpc.call_typed(&GetinfoRequest {}).await?;

        Ok(NodeNames {
            rpc_file,
            own_pubkey: info_res.id,
            channel_peer_cache: CacheBuilder::new(CACHE_SIZE).build(),
            name_cache: CacheBuilder::new(CACHE_SIZE)
                .time_to_live(Duration::from_secs(CACHE_EXPIRY))
                .build(),
        })
    }

    pub async fn get_channel_peer_alias(self, channel_id: &str) -> Result<String, Error> {
        let peer_pubkey = self.clone().get_channel_peer(channel_id).await?;
        self.get_node_alias(peer_pubkey).await
    }

    async fn get_channel_peer(self, channel_id: &str) -> Result<PublicKey, Error> {
        let parsed_id = ShortChannelId::from_str(channel_id)?;

        if let Some(cached_pubkey) = self.channel_peer_cache.get(&parsed_id) {
            return Ok(cached_pubkey);
        }

        let mut rpc = ClnRpc::new(self.rpc_file.clone()).await?;
        let channels = rpc
            .call_typed(&ListchannelsRequest {
                short_channel_id: Some(parsed_id),
                source: None,
                destination: None,
            })
            .await?;

        if channels.channels.is_empty() {
            return Err(anyhow!("could not find channel with id: {}", channel_id));
        }
        let channel = &channels.channels[0];

        let peer_pubkey = if channel.source.eq(&self.own_pubkey) {
            channel.destination
        } else {
            channel.source
        };

        self.channel_peer_cache.insert(parsed_id, peer_pubkey);
        Ok(peer_pubkey)
    }

    async fn get_node_alias(self, pubkey: PublicKey) -> Result<String, Error> {
        if let Some(cached_alias) = self.name_cache.get(&pubkey) {
            return Ok(cached_alias);
        }

        let mut rpc = ClnRpc::new(self.rpc_file.clone()).await?;
        let nodes = rpc
            .call_typed(&ListnodesRequest { id: Some(pubkey) })
            .await?;

        if nodes.nodes.is_empty() {
            return Err(anyhow!("could not find node with id: {}", pubkey));
        }
        let node = &nodes.nodes[0];
        let alias = node.alias.clone().unwrap_or_else(|| pubkey.to_string());

        self.name_cache.insert(pubkey, alias.clone());
        Ok(alias)
    }
}
