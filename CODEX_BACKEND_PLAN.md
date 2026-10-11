# Kế hoạch lấy hạn mức Codex trực tiếp cho UsagePin

Ngày khảo sát: 10/10/2026.

Mục tiêu đã xác nhận: tìm cách OmniRoute lấy usage/hạn mức trực tiếp từ Codex và lập kế hoạch áp dụng cho UsagePin. Tài liệu này là kết quả khảo sát mã nguồn; chưa triển khai hoặc gọi endpoint bằng tài khoản thật.

Giả định: bản đầu theo dõi một tài khoản Codex đã đăng nhập bằng ChatGPT trên máy Windows. Nguồn cần lấy là phần trăm hạn mức và giờ reset. Credential chỉ được xử lý trong backend Rust.

## 1. Cơ chế đã xác định trong OmniRoute

Mã nguồn đối chiếu tại commit `fc5e2bccd4f70fecf5aab94dfb8136c74ab5a21b` của [diegosouzapw/OmniRoute](https://github.com/diegosouzapw/OmniRoute/tree/fc5e2bccd4f70fecf5aab94dfb8136c74ab5a21b).

Luồng HTTP trực tiếp:

```text
Phiên Codex/ChatGPT → access token + account/workspace ID
                  → GET chatgpt.com/backend-api/wham/usage
                  → chuẩn hóa các cửa sổ hạn mức
                  → phần trăm còn lại + thời điểm reset
```

### Request

[`open-sse/services/usage/codex.ts`](https://github.com/diegosouzapw/OmniRoute/blob/fc5e2bccd4f70fecf5aab94dfb8136c74ab5a21b/open-sse/services/usage/codex.ts) gọi:

```http
GET https://chatgpt.com/backend-api/wham/usage
Authorization: Bearer <access_token>
Accept: application/json
Content-Type: application/json
chatgpt-account-id: <account_id>
```

OmniRoute thêm `User-Agent`, `originator` và `Version` qua [`getCodexBackendIdentityHeaders`](https://github.com/diegosouzapw/OmniRoute/blob/fc5e2bccd4f70fecf5aab94dfb8136c74ab5a21b/open-sse/config/codexClient.ts). Đây là headers quan sát được trong implementation; chưa chứng minh mỗi header đều bắt buộc cho UsagePin.

Account ID được lấy từ workspace đã lưu. UsagePin nên ưu tiên `tokens.account_id` của phiên hiện có và không tự chuyển sang workspace khác nếu request thất bại.

### Credential

OmniRoute hỗ trợ OAuth riêng và nhập file phiên Codex. [`codexAuthImport.ts`](https://github.com/diegosouzapw/OmniRoute/blob/fc5e2bccd4f70fecf5aab94dfb8136c74ab5a21b/src/lib/oauth/utils/codexAuthImport.ts) đọc `tokens.access_token`, `tokens.refresh_token`, `tokens.id_token`, `tokens.account_id`; khi thiếu account ID, nó lấy metadata từ claim `https://api.openai.com/auth` trong ID token. Decode JWT ở đây chỉ để lấy metadata, không phải xác minh token.

[Tài liệu xác thực OpenAI](https://learn.chatgpt.com/docs/auth) xác nhận credential có thể nằm trong `$CODEX_HOME/auth.json`, mặc định là `~/.codex/auth.json`, hoặc kho credential của hệ điều hành. Vì vậy không thể giả định mọi máy đều có file này. `auth_mode` cũng không nên là trường bắt buộc nếu các token cần thiết đã tồn tại; không dùng phiên chỉ có API key để lấy hạn mức thuê bao ChatGPT.

### Response và cách hiển thị

[`codexUsageQuotas.ts`](https://github.com/diegosouzapw/OmniRoute/blob/fc5e2bccd4f70fecf5aab94dfb8136c74ab5a21b/open-sse/services/codexUsageQuotas.ts) đọc:

| Trường upstream | Dữ liệu UsagePin cần |
| --- | --- |
| `plan_type` | Loại gói khi có |
| `rate_limit.primary_window`, `secondary_window` | Các cửa sổ quota thực tế |
| `used_percent` | Đã dùng; còn lại = `clamp(100 - used_percent, 0, 100)` |
| `limit_window_seconds` | Thời lượng để sinh nhãn |
| `reset_at` | Unix timestamp theo giây |
| `reset_after_seconds` | Fallback: thời điểm nhận response + số giây |
| `limit_reached` | Trạng thái chạm hạn mức |

Các block review, `additional_rate_limits`, credits và banked resets có thể xuất hiện. Bản đầu chỉ hiển thị quota Codex chính; không gộp các bucket khác vào hai thanh hiện tại.

Không mặc định primary luôn là 5 giờ hoặc secondary luôn là tuần. Không sao chép fallback chuyển `used_percent` thiếu thành 0 của parser OmniRoute: trong widget, thiếu dữ liệu phải là unknown, tránh hiển thị sai 100% còn lại.

[`codexQuotaFetcher.ts`](https://github.com/diegosouzapw/OmniRoute/blob/fc5e2bccd4f70fecf5aab94dfb8136c74ab5a21b/open-sse/services/codexQuotaFetcher.ts) dùng cache 60 giây và timeout 8 giây. Đây là giá trị tham khảo cho polling, không phải giới hạn được OpenAI công bố.

## 2. Quyết định về token cho bản đầu

Đề xuất đơn giản nhất: UsagePin đọc phiên Codex hiện có trước mỗi lần gọi, giữ credential trong bộ nhớ backend và chỉ thực hiện GET usage.

Khi gặp 401, đọc lại nguồn credential; nếu access token đã thay đổi thì thử lại một lần. Nếu chưa có token mới hoặc vẫn bị từ chối, hiển thị cần cập nhật phiên/đăng nhập lại trong Codex. 403 phải giữ khả năng là lỗi quyền/workspace; không khẳng định tất cả 403 đều là hết hạn token.

Chưa tự refresh hoặc ghi đè `auth.json` trong bản đầu. Đây là quyết định thiết kế của kế hoạch, không phải mô tả rằng OmniRoute không refresh. OmniRoute có [refresh-token grant](https://github.com/diegosouzapw/OmniRoute/blob/fc5e2bccd4f70fecf5aab94dfb8136c74ab5a21b/open-sse/services/tokenRefresh/providers/codex.ts) và [serialization cho quota refresh](https://github.com/diegosouzapw/OmniRoute/blob/fc5e2bccd4f70fecf5aab94dfb8136c74ab5a21b/src/lib/usage/providerLimits/credentialRefresh.ts). Các đường import của nó cũng có hành vi khác nhau: single-file import ghi rõ tránh refresh ngay, trong khi bulk import có bước refresh để kiểm tra phiên. Không sao chép nguyên cả luồng import vào widget.

Mã OmniRoute ghi nhận lỗi token xoay vòng như `refresh_token_reused` và `invalid_grant`. Một mutex riêng trong UsagePin không phối hợp được với Codex CLI/desktop đang dùng cùng phiên. Hạn chế của bản đầu: nếu Codex không cập nhật credential, UsagePin không tự duy trì phiên hết hạn.

Nếu cần tự duy trì phiên về sau, phải chốt riêng cơ chế sở hữu credential: dùng Codex quản lý auth qua app-server hoặc đăng nhập riêng và lưu token riêng. Không tự mở rộng bản đầu sang OAuth riêng.

## 3. UsagePin hiện tại và phạm vi thay đổi dự kiến

- `frontend/UsagePin.tsx:11`: hai hàng usage cố định, có nhãn `SAMPLE DATA`.
- `frontend/shared/useRefreshInterval.ts`: lưu chu kỳ refresh trong localStorage, mặc định 60 giây, chấp nhận tối thiểu 1 giây; chưa có polling.
- `frontend/settings/useSettings.ts` và `Settings.tsx`: đã có điều khiển chu kỳ refresh.
- `backend/src/lib.rs`: đã có command/state Tauri cho cửa sổ; chưa có nguồn usage.
- `backend/Cargo.toml`: đã có serde/serde_json, chưa có HTTP client.

Khi triển khai, thêm `backend/src/codex_usage.rs`, dependency HTTP/TLS cần thiết, state và command Tauri. Thêm hook frontend nhận snapshot, thay dữ liệu mẫu và nối tùy chọn refresh. CSS chỉ bổ sung nếu cần thể hiện trạng thái dữ liệu.

Hàm mẫu `greet` tồn tại trước và hiện không được frontend gọi; không xóa trong công việc này. Giữ nguyên các chỉnh sửa đang có trong working tree.

## 4. Các bước triển khai và tiêu chí kiểm chứng

### Bước A — Kiểm chứng nguồn thực tế

1. Xác định credential store và `CODEX_HOME` thực tế, không ghi token ra log.
2. Thử một GET usage bằng access token hiện có, account ID đúng và headers đã đối chiếu.
3. Lưu fixture đã loại credential/thông tin nhận diện; đối chiếu với cùng tài khoản/workspace trong Codex ở thời điểm gần nhau.
4. Ghi lại response thật, status lỗi và headers cần thiết; không gọi model chỉ để đo quota.

**Đạt khi:** có phần trăm đã dùng và reset thật của đúng tài khoản; nhận diện được chưa đăng nhập, credential không đọc được và phiên API-key-only. Nếu chỉ có keyring, xác định cách đọc tương thích trước khi cam kết hỗ trợ; bản chỉ đọc file phải báo rõ không hỗ trợ nguồn đó.

Đây là bước cần thực hiện sau khi được yêu cầu triển khai/kiểm chứng. Lượt khảo sát này chưa đọc credential cá nhân hoặc thử endpoint bằng phiên thật.

### Bước B — Backend Rust

- HTTP client có timeout; không chuyển tiếp bearer token qua redirect sang host khác.
- Chuẩn hóa snapshot: trạng thái, thời điểm lấy dữ liệu, các cửa sổ với `usedPercent`, `windowSeconds`, `resetsAt` nullable; frontend không nhận token/raw auth document.
- Command dự kiến: `get_codex_usage`, `refresh_codex_usage`, `set_usage_refresh_interval`; event `codex-usage-updated`.
- Chỉ backend sở hữu scheduler: chống request chồng nhau, chu kỳ mặc định 60 giây, đọc lại sau wake/reset. Tôn trọng cấu hình refresh hiện có; 429 có `Retry-After` thì ưu tiên thời gian đó, lỗi mạng/5xx có backoff.
- Phân loại 401, 403, 429, timeout, dữ liệu sai schema; dữ liệu cũ có timestamp rõ. Cache không được dùng lại khi tài khoản/workspace thay đổi.

**Đạt khi:** fixture đã dùng 25% cho remaining 75%; window thiếu không thành 100%; timestamp tính đúng đơn vị; một lúc chỉ có một request; đổi tài khoản không hiện cache cũ; 401 chỉ retry khi đọc được token mới.

### Bước C — Nối widget

- Thay `usageRows` mẫu bằng snapshot; sinh nhãn theo thời lượng thực tế.
- Hiển thị giờ reset theo múi giờ máy; khi tới giờ reset thì fetch lại, không tự đặt remaining thành 100%.
- Thể hiện loading, chưa đăng nhập, fresh, stale và lỗi; trường thiếu hiển thị dấu `—`.
- Nối giá trị refresh đã lưu vào scheduler backend cả khi khởi động và khi Settings thay đổi; trả lỗi cập nhật scheduler cho UI nếu có.

**Đạt khi:** thay đổi nguồn được phản ánh lên widget; một window vẫn hiển thị đúng; thay chu kỳ có hiệu lực; mất mạng không biến dữ liệu cũ thành mới; compact view và Settings hiện tại tiếp tục hoạt động.

### Bước D — Kiểm tra hoàn tất

Chạy parser/error/scheduler tests có ý nghĩa, `npm run build`, `cargo check` và test Rust liên quan. Kiểm chứng tài khoản thật, đổi phiên, mất mạng và sleep/wake trên Windows. Kiểm tra log không chứa credential.

Mỗi mục tiêu triển khai phải được xác nhận trước khi bắt đầu. Tối đa ba vòng sửa để đạt tiêu chí; nếu không đạt thì báo nguyên nhân và phần chưa kiểm chứng. Nếu cần thay nguồn hoặc thay tiêu chí, dừng và xác nhận lại.

## 5. Giới hạn và quan hệ với kế hoạch cũ

`/backend-api/wham/usage` là endpoint backend ChatGPT mà mã OmniRoute sử dụng. Việc đọc mã nguồn chứng minh cơ chế của OmniRoute; chưa chứng minh endpoint hoạt động với phiên hoặc môi trường mạng trên máy này. Schema/headers có thể thay đổi, vì vậy Bước A là điều kiện trước khi tích hợp.

`FEATURE_PLAN.md` đã đề xuất Codex app-server. [Tài liệu OpenAI về app-server](https://learn.chatgpt.com/docs/app-server) công bố `account/rateLimits/read` và sự kiện cập nhật. Đường đó vẫn là lựa chọn nếu cần Codex quản lý credential/keyring, nhưng kế hoạch này nghiên cứu đường HTTP trực tiếp theo yêu cầu. Không tự chuyển sang app-server khi kiểm chứng HTTP thất bại và không sửa roadmap cũ trong lượt này.
