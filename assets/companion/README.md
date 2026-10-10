# 두들 컴패니언 이미지 제작

내장 image_gen 도구로 생성한 프로젝트 전용 이미지입니다. `concept.png`는 초기 외형 참고이고, `body.png`는 앱이 실제 사용하는 눈 분리 레이어입니다. 눈은 Rust에서 같은 기울기의 검은 캡슐 두 개로 그려 시선과 눈 깜박임을 움직입니다.

참고: 사용자가 지정한 ChatGPT 프로젝트 **Grok Bot 스타일 이미지 생성**의 **지침 수정 검토** 대화(2026-10-08)의 Minimal Bot Icon Converter v2.0. 둥근 얼굴, 검은 캡슐 눈(2.7:1), 입·코·눈썹 없음, 작은 홍조, 시계 방향 기울기, 단순한 색면을 따릅니다. 애니메이션용으로 차콜 배경/가장자리 크롭 대신 전체 실루엣과 투명 배경을 사용했습니다. 캐릭터는 새로 만든 살구색 잉크방울이며 Coucou 캐릭터 자산은 사용하지 않습니다.

## 최초 생성 프롬프트

Use case: stylized-concept.
Asset type: original animated desktop assistant character for Doodle Font, a friendly doodling app. Create ONE final mascot head, no multiple views.
Design reference rules from the user's "Minimal Bot Icon Converter v2.0": minimalist flat 2D, smoothly rounded face, exactly two identical solid near-black #111111 vertical capsule eyes (parallel straight sides, semicircular ends, height:width 2.7:1), no nose, no mouth, no eyebrows, no highlights or gradients. Eye length is 19% of face width, center separation 38% face width. Two tiny low-saturation peach oval blush marks below the outer eyes, each about half eye length. Large simple color masses, no outlines, no text, no symbols, no particles.
Original character: a warm pale-apricot rounded ink-drop friend, a single little soft curl on top like a brushstroke. Face color #F5CFAB, curl a single matte terracotta #C97554 mass. No robot parts, limbs, body, ears, hats, tools or clothing. Not a copy of Coucou/Mochi.
Composition: near-front face, head/eyes/blush all rotated 18 degrees CLOCKWISE (left eye higher than right). A welcoming quiet curious feeling expressed only by the tilt. Adapt the reference avatar cropping/background to this functional animated UI sprite: entire silhouette visible with generous transparent margin, square image, centered, real alpha transparency, no background, no shadows. Silhouette occupies about 80% of the image. Flat solid colors only, exceptionally clean edges, small-icon readability.

## 눈 분리 레이어 프롬프트

Edit this character into a production animation BODY layer for the Doodle Font app. Preserve exact silhouette, curl, face geometry, blush positions, colors, camera, pose, transparent margin and alpha. Remove BOTH black eyes completely and fill their former area seamlessly with the face color: the app will draw animated capsule eyes as a separate runtime layer, so no eye pixels, no facial marks except the two existing blush ovals. Also correct the original's unwanted shading: make the ENTIRE FACE precisely one uniform solid #F5CFAB, the ENTIRE CURL one uniform solid #C97554, each blush one uniform solid #EFA88A. Absolutely no texture, gradient, lighting, shading, gloss, outline, mouth or nose. Crisp smooth alpha edges, no white fringe. Do not redesign, rotate, move, resize or crop any component. Return the single transparent character body layer in the same square canvas.
