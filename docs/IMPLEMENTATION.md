# 두들 폰트 구현 계획

사용자 결정: Windows 데스크톱 앱. 사용자 제공 요구사항은 `requirements.ko.txt`에 보존한다.

1. Git 초기화, 요구사항 보존, egui/eframe + Rust 네이티브 폰트 엔진 기반 구성.
2. 원문/지역별 글리프/템플릿 원본/승인본/AI 후보 분리. 자소 단위 추출, 문자 집합, PUA, 공백 및 제어 문자 보호.
3. 벡터·비트맵·레이어 편집, 부품 재사용, 복원, 프로젝트 저장. 한국어 작업실 UI.
4. 실제 TTF 출력(cmap, format 14, GSUB, locl, COLR/CPAL, sbix), 범위·분할 출력 및 실제 폰트 검증.
5. 선택적인 OpenAI/Anthropic 호환 API 및 로컬 멀티모달 AI, 템플릿 권한 관리.
6. 도메인/출력/데스크톱 통합 테스트, Windows 실행 파일 패키징 및 단계별 커밋.

모든 편집 데이터는 로컬에 보존한다. API 연결 정보와 키는 프로젝트 밖의 설정 파일에 현재 Windows 사용자 범위의 DPAPI로 암호화해 저장한다. 인증창을 금지하는 옵션을 사용해 다음 실행에 자동 복원하며, 키는 제공자·API 주소별로 구분한다. API 후보는 사용자가 채택해야 적용되며 잠금·의도적 빈 글리프·미지원 항목을 자동 수정하지 않는다. 시스템 렌더링은 폰트에 복제하지 않는다.

참고 사양:
- https://www.unicode.org/reports/tr29/
- https://learn.microsoft.com/en-us/typography/opentype/spec/cmap
- https://learn.microsoft.com/en-us/typography/opentype/spec/features_ko
- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create
- https://platform.claude.com/docs/en/api/messages/create
- https://huggingface.co/Cactus-Compute/needle3
- https://huggingface.co/google/gemma-4-E2B-it
- https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md

사용자 최종 결정: Rust 기반, Tauri 제외. egui/eframe 네이티브 UI를 사용한다. Node.js, Electron, Tauri, WebView2, Python은 앱 실행 및 빌드에 필요하지 않다. Python fontTools는 출력물을 독립적으로 확인하는 개발용 검증에만 사용한다.

로컬 생성 AI는 Gemma 4 E2B 단독이며 llama.cpp b11531을 사용한다. Needle 3는 별도 설치와 켜기/끄기를 가진 공통 입력 판단 옵션이다. 입력 직후 판단 → Rust의 실행 전 확인 → 빠른 편집 후보 또는 선택한 로컬/OpenAI/Anthropic 생성 AI 순서로 처리한다. 원래 요청·대상·모드·이미지 전송 동의를 보존하며, 실패/불확실한 판단은 원문을 전달하고 취소는 전체 작업을 중단한다. 기본 Needle의 한국어 성능은 제한적이므로 기본값은 꺼짐이며 시험 기능으로 표시한다. 하단 플로팅 입력창은 AI 준비 시 표시하며 작업 중 모든 편집을 잠그고 상태·취소만 제공한다. 작업 결과는 후보로 분리되며 완료·실패·취소 시 편집을 다시 연다.

## 한국어 입력 판단 모델 조사 · 2026-10-10

이번 변경은 조사까지이며 Needle 교체나 새 모델 설치는 하지 않았다. 아래 용량은 공식 저장소의 **모델 파일 크기**로 실행 중 RAM·설치 총량·처리 속도가 아니다. 한국어 지원이나 일반 벤치마크만으로 두들 폰트 명령의 정확도를 확정하지 않는다.

| 후보 | 확인한 모델 파일 | 한국어 및 적용 방식 | 평가 |
| --- | --- | --- | --- |
| KoELECTRA-Small-v3 discriminator | FP32 약 56.6MB | 한국어 사전학습 모델. 색상/굵기/생성/상담/불명확 등 명령 분류용 추가 학습 필요 | 작은 메모리 목표에 가장 가까운 개발 방향. 설치만으로 Needle의 함수 호출을 대체하지는 못함 |
| multilingual-e5-small | 공식 INT8 ONNX 약 118.3MB | 한국어를 포함한 100개 언어. 예문과 입력의 의미 유사도로 의도 분류 구성 | 학습 없이 시작할 평가 후보. 함수 호출/인자 추출 모델이 아니므로 숫자·부정·복합 명령 처리는 별도로 필요 |
| Qwen3-0.6B | 공식 Q8 GGUF 약 639.4MB | 한국어 포함 119개 언어·방언. 비사고 모드와 도구 호출 지원 | 자유로운 표현과 인자 추출 평가 후보지만 Needle보다 모델 파일이 훨씬 큼. 매 입력마다 생성형 모델을 호출하는 비용 확인 필요 |
| FunctionGemma 270M | 이번 조사에서 배포 형식별 용량 미확정 | Google이 특정 함수 호출 작업에 맞춘 추가 학습을 권장 | 한국어 판단 성능이 확인된 즉시 교체품으로 간주하지 않음 |
| Qwen3.5-0.8B | 이번 조사에서 배포 형식별 용량 미확정 | 공식 문서는 201개 언어·방언, 소형 모델의 작업별 학습/연구 용도를 안내 | 후속 비교 후보. 한국어 글리프 명령 정확도와 현재 실행기의 호환성·속도는 미측정 |

권장 순서는 제한된 한국어 명령 예문으로 E5의 분류 가능성을 먼저 평가하고, 메모리·지연 목표가 엄격하면 KoELECTRA-Small에 명령 분류를 학습하는 것이다. 이는 모델 구조와 배포 크기에 따른 설계 판단이며 실제 성능 비교 결과는 아니다. 임베딩 유사도를 실행 확률로 해석하지 말고 검증셋에서 거절 기준을 보정한다. 기존의 Rust 검증과 원문 전달 경로는 유지하며, `빨간색으로 바꿔`, `빨간색으로 바꾸지 마`, `굵기만 70으로`, 여러 조건/오타/띄어쓰기/영한 혼합/지원하지 않는 명령을 포함해 정확도·잘못된 실행률·처음/반복 요청 지연·최대 메모리를 측정해야 한다.

공식 근거:

- [KoELECTRA 개발자 설명 및 한국어 과제 평가](https://github.com/monologg/KoELECTRA), [Small-v3 가중치 56,577,499바이트](https://huggingface.co/monologg/koelectra-small-v3-discriminator/blob/main/pytorch_model.bin).
- [multilingual-e5-small 모델 설명](https://huggingface.co/intfloat/multilingual-e5-small), [공식 INT8 ONNX 118,346,824바이트](https://huggingface.co/intfloat/multilingual-e5-small/blob/main/onnx/model_qint8_avx512_vnni.onnx). 이 파일은 AVX512 VNNI용이며 CPU별 배포 호환성과 양자화 방식은 별도 확인한다.
- [Qwen3 모델·언어 설명](https://qwenlm.github.io/blog/qwen3/), [0.6B 비사고 모드·도구 호출](https://huggingface.co/Qwen/Qwen3-0.6B), [공식 Q8 GGUF 639,446,688바이트](https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/blob/main/Qwen3-0.6B-Q8_0.gguf).
- [FunctionGemma의 작업별 학습 권장](https://ai.google.dev/gemma/docs/functiongemma), [Qwen3.5-0.8B 공식 모델 설명](https://huggingface.co/Qwen/Qwen3.5-0.8B).
