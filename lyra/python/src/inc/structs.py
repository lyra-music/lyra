import typing as t


class LyraConfig(t.NamedTuple):
    token: str
    prefixes: t.Collection[str]
    emoji_guild: int
