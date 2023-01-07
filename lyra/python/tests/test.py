import pathlib as pl

import lyra


def main():
    # for i, f in enumerate(pl.Path('python/tests/imgs').glob('*')):
    #     with open(f, 'rb') as inf, open(f'python/tmp/out{i}', 'w') as outf:
    #         cs = lyra.get_dominant_palette_from_image(inf.read(), 5)
    #         for c in cs:
    #             outf.write(f'#{c:02x}\n')
    for f in pl.Path('python/tests/imgs').glob('*'):
        _ = lyra.get_dominant_palette_from_image_path(str(f), 5)
        # _ = lyra.limit_image_byte_size(f.open('rb').read(), 8 * 10**6)


if __name__ == '__main__':
    main()
