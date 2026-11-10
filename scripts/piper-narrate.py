"""Generate agent narration without an unbounded ONNX worker pool.

Run with the existing Piper environment's Python. Playback is deliberately
separate: finish synthesis before measuring hardware timing or executing cues.
No voice, user text, output audio or private paths belong in source control.
"""

import argparse
import json
from pathlib import Path
import wave

import onnxruntime
from piper.config import PiperConfig
from piper.voice import PiperVoice


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model", type=Path, required=True)
    parser.add_argument("--input-file", type=Path, required=True)
    parser.add_argument("--output-file", type=Path, required=True)
    parser.add_argument("--threads", type=int, choices=range(1, 5), default=1)
    args = parser.parse_args()
    text = args.input_file.read_text(encoding="utf-8").strip()
    if not text:
        parser.error("The UTF-8 input file is empty")
    if args.output_file.resolve() in {
        args.model.resolve(),
        Path(f"{args.model}.json").resolve(),
        args.input_file.resolve(),
    }:
        parser.error("Output must not overwrite the model, configuration or input")

    options = onnxruntime.SessionOptions()
    options.intra_op_num_threads = args.threads
    options.inter_op_num_threads = 1
    options.execution_mode = onnxruntime.ExecutionMode.ORT_SEQUENTIAL
    config = PiperConfig.from_dict(
        json.loads(Path(f"{args.model}.json").read_text(encoding="utf-8"))
    )
    voice = PiperVoice(
        config=config,
        session=onnxruntime.InferenceSession(
            str(args.model), sess_options=options, providers=["CPUExecutionProvider"]
        ),
        download_dir=args.model.parent,
    )
    with wave.open(str(args.output_file), "wb") as output:
        voice.synthesize_wav(text, output)
    print(f"Narration finalized; ONNX intra/inter threads: {args.threads}/1")


if __name__ == "__main__":
    main()
