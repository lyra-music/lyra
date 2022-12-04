import os
import pathlib as pl

import yaml
import hikari as hk
import tanjun as tj
import alluka as al

import lavalink as lv

from .inc import LyraConfig


with pl.Path('../config.yml').open('r') as f:
    _d = yaml.load(  # pyright: ignore [reportUnknownMemberType]
        f, Loader=yaml.FullLoader
    )
    lyra_conf = LyraConfig(os.environ['BOT_TOKEN'], _d['prefixes'], _d['emoji_guild'])

activity = hk.Activity(
    name=':)',
    type=hk.ActivityType.LISTENING,
)

client = tj.Client.from_gateway_bot(
    bot := hk.GatewayBot(lyra_conf.token),
    declare_global_commands=True,
    mention_prefix=True,
)


@client.with_listener()
async def on_started(
    _: hk.StartedEvent, bot: al.Injected[hk.GatewayBot], client: al.Injected[tj.Client]
):
    bot_u = bot.get_me()
    assert bot_u
    bot_id = bot_u.id

    LAVALINK_HOST = '0.0.0.0'
    LAVALINK_PORT = int(os.environ['LAVALINK_PORT'])
    LAVALINK_PWD = os.environ['LAVALINK_PWD']

    lvc = lv.Client(bot_id)
    lvc.add_node(LAVALINK_HOST, LAVALINK_PORT, LAVALINK_PWD, 'us', name='lyra-lavalink')

    client.set_type_dependency(lv.Client, lvc)


@client.with_listener()
async def on_shard_payload(event: hk.ShardPayloadEvent, client: al.Injected[tj.Client]):
    if not (event.name == 'VOICE_STATE_UPDATE' or event.name == 'VOICE_SERVER_UPDATE'):
        return

    lvc = client.get_type_dependency(lv.Client)
    assert lvc
    await lvc.voice_update_handler({'t': event.name, 'd': event.payload})
