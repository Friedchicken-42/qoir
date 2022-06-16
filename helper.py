from PIL import Image
import sys

x = 'RGBA'

if sys.argv[1].endswith('.png'):
    im = Image.open(sys.argv[1]).convert(x)
    w, h = im.size
    print(im.size)
    with open("a.raw", 'wb') as f:
        for p in im.getdata():
            for i in p:
                f.write(i.to_bytes(1, 'big'))

elif sys.argv[1].endswith('.raw'):
    if len(sys.argv) != 4:
        print('use helper.py (image.raw) (width) (height)')
        exit()

    with open('b.raw', 'rb') as f:
        content = f.read()
        w = int(sys.argv[2])
        h = int(sys.argv[3])
        print(w, h)
        im = Image.frombuffer(x, (w, h), content, 'raw', x, 0, 1)
        im.save('b.png')
