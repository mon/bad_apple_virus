from itertools import cycle, product
import os
import json
from typing import Optional
from PIL import Image, ImageDraw
import cv2
from tqdm import tqdm
import numpy as np
import argparse

def parse_args():
    parser = argparse.ArgumentParser(description="Process a video and extract significant regions from each frame.")
    parser.add_argument("--input_video", type=str, default="input.webm", help="Path to the input video file.")
    parser.add_argument("--output_dir", type=str, default="output_frames", help="Directory to save the output images.")
    parser.add_argument("--max_width", type=int, default=64, help="Maximum width for processed frames.")
    parser.add_argument("--threshold", type=float, default=0.4, help="Threshold for binarization (as a fraction of 255).")
    parser.add_argument("--boxes_file", type=str, default="boxes.json", help="Path to save the boxes data.")
    return parser.parse_args()

def load_json(file_path):
    with open(file_path) as f:
        return json.load(f)

def write_binary_data(file_path, data):
    with open(file_path, "wb") as f:
        for frame in data:
            for window in frame:
                f.write(bytes(window))
            f.write(bytes([0, 0, 0, 0]))

def process_image(image: Image, max_width: int, threshold: float, output_dir: str, image_counter: int) -> list:
    w, h = image.size
    ratio = w / h

    # greyscale
    image = image.convert("L")
    # resize
    image = image.resize((max_width, int(max_width / ratio)))
    # threshold
    image = image.point(lambda p: 255 if p > threshold else 0)
    # mono
    image = image.convert("1")

    # find largest region via brute force
    # tqdm.write(f'{image.width=} {image.height=}')
    pixels = image.load()
    visited = np.zeros(image.size, dtype=bool)

    # visualisation
    boxes = []
    work = image.copy().convert("RGB")
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

        for x, y in product(range(image.width), range(image.height)):
            if visited[x, y] or pixels[x, y] == 0:
                visited[x, y] = True
                continue

            sublargest: Optional[tuple[int, int]] = None
            widest = image.width - x  # optimise

            if widest == 0:
                continue

            # row by row
            for h in range(image.height - y):
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

    # image.show()
    # work.show()

    work.save(os.path.join(output_dir, f"{image_counter}.png"))

    return boxes

def process_video(input_video: str, max_width: int, threshold: float, output_dir: str):
    cap = cv2.VideoCapture(input_video)
    total_frames = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))
    prog = tqdm(total=total_frames)
    all_boxes = []
    image_counter = 0

    try:
        while cap.isOpened():
            ret, cv2_im = cap.read()
            if ret:
                converted = cv2.cvtColor(cv2_im, cv2.COLOR_BGR2RGB)
                pil_im = Image.fromarray(converted)
                boxes = process_image(pil_im, max_width, threshold, output_dir, image_counter)
                all_boxes.append(boxes)
                image_counter += 1
                prog.update()
            else:
                break
    finally:
        cap.release()
        return all_boxes

def save_boxes_json(file_path, data):
    with open(file_path, "w") as f:
        json.dump(data, f)

def main():
    args = parse_args()

    # Load and process JSON data
    json_data = load_json("assets/boxes.json")
    # Perform analysis and print statements using json_data

    # Process video and save boxes
    all_boxes = process_video(args.input_video, args.max_width, args.threshold * 255, args.output_dir)
    save_boxes_json(args.boxes_file, all_boxes)

if __name__ == "__main__":
    main()
