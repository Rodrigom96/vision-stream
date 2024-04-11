import argparse
import cv2
from vision_stream.deepstream import NvRtspSource


def main(rtsp_path: str):
    cap = NvRtspSource(rtsp_path)

    # Read until video is completed
    while True:
        # Capture frame-by-frame
        frame = cap.read()
        if frame is not None:
            # Display the resulting frame
            cv2.imshow('Frame', frame.to_tensor().cpu().numpy())

        if cap.is_reconnecting():
            print("reconnecting...")
            if cv2.waitKey(1000) & 0xFF == ord('q'):
                break

        # Press Q on keyboard to  exit
        if cv2.waitKey(25) & 0xFF == ord('q'):
            break


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("rtsp_path")

    args = parser.parse_args()

    print(f"RTSP: {args.rtsp_path}")
    main(args.rtsp_path)
