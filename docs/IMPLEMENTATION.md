# 두들 폰트 구현 계획

사용자 결정: Windows 데스크톱 앱. 사용자 제공 요구사항은 `requirements.ko.txt`에 보존한다.

1. Git 초기화, 요구사항 보존, egui/eframe + Rust 네이티브 폰트 엔진 기반 구성.
2. 원문/지역별 글리프/템플릿 원본/승인본/AI 후보 분리. 자소 단위 추출, 문자 집합, PUA, 공백 및 제어 문자 보호.
3. 벡터·비트맵·레이어 편집, 부품 재사용, 복원, 프로젝트 저장. 한국어 작업실 UI.
4. 실제 TTF 출력(cmap, format 14, GSUB, locl, COLR/CPAL, sbix), 범위·분할 출력 및 실제 폰트 검증.
5. 선택적인 OpenAI/Anthropic 호환 API 후보 생성, 템플릿 권한 관리.
6. 도메인/출력/데스크톱 통합 테스트, Windows 실행 파일 패키징 및 단계별 커밋.

모든 편집 데이터는 로컬에 보존한다. API 키는 메모리에만 유지한다. API 후보는 사용자가 채택해야 적용되며 잠금·의도적 빈 글리프·미지원 항목을 자동 수정하지 않는다. 시스템 렌더링은 폰트에 복제하지 않는다.

참고 사양:
- https://www.unicode.org/reports/tr29/
- https://learn.microsoft.com/en-us/typography/opentype/spec/cmap
- https://learn.microsoft.com/en-us/typography/opentype/spec/features_ko
- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create
- https://platform.claude.com/docs/en/api/messages/create
- https://www.electronjs.org/docs/latest/tutorial/security

사용자 최종 결정: Rust 기반, Tauri 제외. egui/eframe 네이티브 UI를 사용한다. Node.js, Electron, Tauri, WebView2, Python은 앱 실행 및 빌드에 필요하지 않다. Python fontTools는 출력물을 독립적으로 확인하는 개발용 검증에만 사용한다.
