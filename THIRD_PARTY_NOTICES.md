# Third-party notices

VRCS distributes open-source runtime components. Their original copyright
notices and license terms remain applicable.

- Tauri — MIT OR Apache-2.0
- React and React DOM — MIT
- react-markdown and remark-gfm — MIT
- pulldown-cmark — MIT
- Axum and Tokio — MIT
- rusqlite and SQLite — MIT / Public Domain
- Vulkan Loader — Apache-2.0
- llama.cpp (Qwen ASR runtime) — MIT
- LLVM OpenMP runtime — Apache-2.0 WITH LLVM-exception
- wasapi-rs — MIT
- Silero VAD — MIT
- Smart Turn — BSD-2-Clause
- ONNX Runtime — MIT
- PaddleOCR PP-OCRv6 models and dictionary — Apache-2.0
- PaddleOCR Korean PP-OCRv5 mobile recognizer and dictionary — Apache-2.0
- PaddleX OCR processing reference — Apache-2.0
- rosc — MIT OR Apache-2.0
- rust-openvr and openvr-sys — MIT
- Valve OpenVR SDK — BSD-3-Clause

The Qwen ASR model and audio projector are downloaded through recognition settings
and remain subject to their upstream Apache-2.0 license. The llama.cpp runtime
is downloaded on demand through recognition settings. It includes its MIT
license and the LLVM OpenMP license.
The bundled Vulkan loader includes the VulkanRT license notices.
The Smart Turn model is downloaded only when semantic endpointing is enabled
and remains subject to the upstream BSD-2-Clause license.
The PP-OCRv6 small models and dictionary are downloaded through the OCR settings
and remain subject to the upstream Apache-2.0 license. Local image processing
follows the PaddleX reference algorithms, Copyright PaddlePaddle Authors.

The optional Korean OCR bundle uses PaddlePaddle's official
`korean_PP-OCRv5_mobile_rec_onnx` export at revision
`5c6f574b8e2230adf4287b33e736d71b9fabd28e`, with the matching
`ppocrv5_korean_dict.txt` from PaddleOCR revision
`b03f46425e8ff4442b268ce449e3eef758146cd4`. It reuses the PP-OCRv6 small detector.
These assets are downloaded only when requested in OCR settings, retain their
upstream Apache-2.0 license, and are verified against pinned SHA-256 digests.
Sources: https://huggingface.co/PaddlePaddle/korean_PP-OCRv5_mobile_rec_onnx
and https://github.com/PaddlePaddle/PaddleOCR (Copyright PaddlePaddle Authors).

The optional Korean dictionary catalog downloads the KO-ZH.KRDICT.zip and
KO-JA.KRDICT.zip Yomitan conversions from Lyroxide's pinned 1.0.0 release:
https://github.com/Lyroxide/yomitan-ko-dic/releases/tag/1.0.0 . These archives are
not bundled with VRCS; they are fetched only when requested and checked against
their pinned size and SHA-256 before local import. The underlying Korean
Learners' Dictionary text is provided by the National Institute of Korean
Language and remains licensed under CC BY-SA 2.0 KR. Yomitan conversion is by
Lyroxide. VRCS imports text and corrects the known language metadata for these
two catalog packages; manually imported archives retain their own metadata.
Dictionary source and copyright policy:
https://krdict.korean.go.kr/kor/kboardPolicy/copyRightTermsInfo .
License: https://creativecommons.org/licenses/by-sa/2.0/kr/ .
Images, audio and video may have separate terms; this importer does not download
or import those media.
