import lyra

img_path = 'python/tests/__maria_marionette_and_sir_ventrilo_nijisanji_and_1_more_drawn_by_39daph__da437b4eafa6cab7728da5f630ad6876.jpg'
result = lyra.get_dominant_palette_from_image(img_path, 8)
with open('python/tmp/out', 'w+') as f:
    for c in result:
        f.write(f"#{'{:06x}'.format(c)[:-2]}\n")
