# based on minimum window size, 64x64 pixels is viable

from itertools import cycle, product
import os
import json
from typing import Optional
from PIL import Image, ImageDraw
import cv2
from tqdm import tqdm
import numpy as np
import argparse

parser = argparse.ArgumentParser(
    prog="Boxes utility")

parser.add_argument('-i', '--input', action='store')
args = parser.parse_args()

user_option = 0

print("Boxes conversion utility")
if not args.input:
    print("Select an option:")
    print("(1) Convert video to boxes.json")
    print("(2) Convert boxes.json to boxes.bin")
    user_option = int(input("Option: "))
else:
    user_option = 1

# invalid options
if user_option > 2 or user_option < 1:
    print("Invalid option")
    exit()

if user_option == 2:
    # checks and such
    with open("assets/boxes.json") as f:
        j = json.load(f)
    print(f"Most visible windows: {max(len(b) for b in j)}")
    print(f"Total frames: {len(j)}")
    print(f"Total window changes: {sum(len(b) for b in j)}")
    print(
        f"Base width: {max(max((coords[0]+coords[2] for coords in b), default=0) for b in j)} Base height: {max(max((coords[1]+coords[3] for coords in b), default=0) for b in j)}"
    )

    print("Serialising box-o'-bytes to boxes.bin")
    with open("assets/boxes.bin", "wb") as f:
        for frame in j:
            for window in frame:
                f.write(bytes(window))
                # null window signifies new frame
            f.write(bytes([0, 0, 0, 0]))
    exit()

# whole arrays printed, debug
# np.set_printoptions(threshold=np.inf)

# try to use the arg inputs, otherwise fallback to user input
inp = args.input or input("Input video: ")
out = "apple_frames"
max_width = 64
threshold = 255 * 0.4

if not os.path.isdir(out):
    print("Cannot find output folder, creating one...")
    os.makedirs(out)


def frame_to_boxes(im: Image, name):
    w, h = im.size
    ratio = w / h

    # greyscale
    im = im.convert("L")
    # resize
    im = im.resize((max_width, int(max_width / ratio)))
    # threshold
    im = im.point(lambda p: 255 if p > threshold else 0)
    # mono
    im = im.convert("1")

    # find largest region via brute force
    # tqdm.write(f'{im.width=} {im.height=}')
    pixels = im.load()
    visited = np.zeros(im.size, dtype=bool)

    # visualisation
    boxes = []
    work = im.copy().convert("RGB")
    draw = ImageDraw.Draw(work)
    fills = cycle(
        [
            "red",
            "green",
            "blue",
            "orange",
            "yellow",
            "purple",
            "pink",
            "cyan",
            "gray",
            "brown",
            "maroon",
            "hotpink",
            "gold",
            "chocolate",
            "green",
        ]
    )

    while False in visited:
        largest: Optional[tuple[int, int, int, int]] = None  # x, y, width, height

        for x, y in product(range(im.width), range(im.height)):
            if visited[x, y] or pixels[x, y] == 0:
                visited[x, y] = True
                continue

            sublargest: Optional[tuple[int, int]] = None
            widest = im.width - x  # optimise

            if widest == 0:
                continue

            # row by row
            for h in range(im.height - y):
                # search until black pixel
                for w in range(widest + 1):
                    if (
                        (w == widest)
                        or visited[x + w, y + h]
                        or pixels[x + w, y + h] == 0
                    ):
                        break

                # tqdm.write(f'tapped out {x} {y} {w} {h} {widest}')

                widest = min(widest, w)
                if sublargest is None or (sublargest[0] * sublargest[1]) < (
                    (w) * (h + 1)
                ):
                    sublargest = [w, h + 1]

            if largest is None or (largest[2] * largest[3]) < (
                sublargest[0] * sublargest[1]
            ):
                largest = [x, y, *sublargest]

            # break # debug

        # tqdm.write(f'{largest=}')

        # Generally only occurs when the entire frame is black
        if largest is None:
            break

        visited[
            largest[0] : largest[0] + largest[2], largest[1] : largest[1] + largest[3]
        ] = True

        boxes.append(largest)

        # [(x0, y0), (x1, y1)] from [x0, y0, w, h], where the bounding box is inclusive
        box = [
            (largest[0], largest[1]),
            (largest[0] + largest[2] - 1, largest[1] + largest[3] - 1),
        ]
        draw.rectangle(box, fill=next(fills))

        # work.show() # debug
        # exit()

        # break # debug

    tqdm.write(f"{len(boxes)=}")

    # im.show()
    # work.show()

    work.save(os.path.join(out, f"{name}.png"))

    return boxes


image_counter = 0

cap = cv2.VideoCapture(inp)
prog = tqdm(total=6570)
all_boxes = []

try:
    while cap.isOpened():
        ret, cv2_im = cap.read()
        if ret:
            converted = cv2.cvtColor(cv2_im, cv2.COLOR_BGR2RGB)

            pil_im = Image.fromarray(converted)
            all_boxes.append(frame_to_boxes(pil_im, f"{image_counter}"))
            image_counter += 1
            prog.update()
        elif not ret:
            break

    cap.release()
finally:
    with open("assets/boxes.json", "w") as f:
        json.dump(all_boxes, f)

# im = Image.open('bad apple.jpg')
# frame_to_boxes(im, 'test')
