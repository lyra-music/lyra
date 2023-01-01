# pyright: reportShadowedImports=false
import os
import dotenv

dotenv.load_dotenv('../.env')  # pyright: ignore [reportUnknownMemberType]

from src import bot, activity


if __name__ == '__main__':
    if os.name != 'nt':
        import uvloop

        uvloop.install()

    bot.run(activity=activity)
