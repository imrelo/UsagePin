# Kiểm tra endpoint usage/quota

Ngày kiểm tra: 11/10/2026, múi giờ Asia/Saigon. Timestamp chính xác và kết quả đã loại thông tin đăng nhập nằm trong `PROVIDER_ENDPOINT_CHECK.json`.

Mục tiêu đã xác nhận: thử endpoint usage/quota bằng phiên đã lưu trong `C:\Users\mdbre`, không gọi model. Người dùng đã phê duyệt rõ việc đọc/giải mã credential và gửi request tới Cursor, Anthropic và các endpoint Google nêu dưới đây. Không refresh token hoặc thay cấu hình đăng nhập. Script kiểm tra tạm đã được xóa; credential không được ghi vào báo cáo.

## Kết quả

| Dịch vụ | Endpoint và method | Kết quả | Kết luận giới hạn |
| --- | --- | --- | --- |
| Cursor | `POST https://cursor.com/api/dashboard/get-current-period-usage` | **200**, dùng phiên Cursor từ `AppData/Roaming/Cursor/User/globalStorage/state.vscdb` | Đã xác minh lấy được JSON usage từ tài khoản hiện có. |
| Claude Code | `GET https://api.anthropic.com/api/oauth/usage` | **401**, token OAuth đã quá hạn ghi trong OmniRoute | Endpoint có phản hồi; chưa xác minh payload quota bằng phiên còn hiệu lực. |
| Antigravity | `POST https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuota` | **401**, token Google OAuth đã quá hạn ghi trong OmniRoute | Chưa xác minh quota thực tế. |
| Antigravity catalog | `POST https://daily-cloudcode-pa.googleapis.com/v1internal:fetchAvailableModels` | **401**, cùng phiên Google OAuth trên | Chưa xác minh quota hoặc danh mục model; dữ liệu danh mục không mặc nhiên đồng nghĩa mức sử dụng thực tế. |
| Gemini CLI | `POST https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuota` | **401**, token Google OAuth đã quá hạn ghi trong 9router | Phiên Gemini CLI này là nguồn riêng với Antigravity. |
| GitHub Copilot | `GET https://api.github.com/copilot_internal/user` | **401**, không gửi credential | Chưa tìm được GitHub token dùng được trong các file đã kiểm tra; chưa thử có xác thực. Không đọc Windows Credential Manager ngoài phạm vi quét thư mục. |
| Grok, endpoint ứng viên | `POST https://grok.com/rest/rate-limits` | **403**, HTML, không gửi credential | Không xác minh được đây là nguồn quota phù hợp cho Grok Bot. |

Các endpoint Anthropic, Cursor, Copilot và Google trong bảng đều trả 401 ở lượt không gửi credential. HTTP 401 chỉ xác nhận server phản hồi, không chứng minh credential, quyền tài khoản hoặc payload quota tương thích. Lượt đầu trong sandbox bị lỗi DNS; các kết quả HTTP trên được lấy sau khi cho phép truy cập mạng.

## Dữ liệu Cursor đã xác minh

Response có `billingCycleStart`, `billingCycleEnd`, `planUsage`, `spendLimitUsage` và các trường hiển thị. Những trường số đã giữ lại trong fixture:

- `planUsage.totalPercentUsed = 100`
- `planUsage.autoPercentUsed = 100`
- `planUsage.apiPercentUsed = 100`
- `planUsage.totalSpend = 409` — giữ giá trị nguyên bản; chưa chốt đơn vị hiển thị tiền trong UsagePin.

Các giá trị này là response của endpoint, chưa đối chiếu trực quan với dashboard Cursor. Fixture chỉ lưu các trường chọn lọc, không lưu response đầy đủ hoặc thông tin nhận diện tài khoản. Không tự suy ra số tiền hạn mức nếu endpoint không cung cấp trường tương ứng.

## Nguồn phiên và điểm cần lưu ý khi tích hợp

- Claude: file `.claude/.credentials.json` không có access token. Request thử dùng phiên OAuth cũ nhập vào OmniRoute, không chứng minh phiên Claude Code hiện tại dùng cùng tài khoản hoặc cơ chế xác thực. Cần phiên Claude subscription còn hiệu lực hoặc nguồn statusline phù hợp để kiểm chứng tiếp.
- Antigravity và Gemini CLI: các token thử đều quá hạn theo metadata lưu trong router. Chưa gọi token refresh; kết quả 401 không phải bằng chứng các dịch vụ không hỗ trợ quota.
- Copilot: cần một nguồn phiên GitHub phù hợp. Không mặc định chỉ số premium requests cũ; phải kiểm tra payload thực tế và AI Credits theo tài khoản hiện tại.
- Grok Bot: credential đã kiểm tra trong `AppData/Roaming/Grok Bot/sand-secrets.json` có issuer `https://authentication.cursor.sh`, là phiên Cursor. Không gửi credential đó tới Grok/xAI. Tên thư mục không đủ để xác định backend quota.
- Endpoint đang dùng có thể là API nội bộ. Khi triển khai cần giữ trạng thái thiếu phiên, hết hạn, bị từ chối, giới hạn tần suất và response không tương thích riêng biệt; không biến dữ liệu thiếu thành 100% quota còn lại.

## Phạm vi đã hoàn thành

Đã kiểm tra endpoint và tạo báo cáo, chưa sửa mã tích hợp UsagePin. Cursor là nguồn duy nhất đã trả usage có xác thực trong lượt kiểm tra này. Những dịch vụ còn lại cần phiên phù hợp hoặc xác định thêm nguồn quota trước khi xác nhận hỗ trợ dữ liệu thật.

## Bổ sung: Claude Desktop

Theo yêu cầu kiểm tra Claude Desktop, đã tìm được dữ liệu app Microsoft Store tại:

`C:\Users\mdbre\AppData\Local\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Roaming\Claude`

`config.json` có cache OAuth riêng ở `oauth:tokenCacheV2`, được mã hóa bằng Electron safeStorage/Windows DPAPI. Đã đọc cache trong bộ nhớ và chọn token khớp `lastKnownAccountUuid`, có scope `user:profile` và `user:sessions:claude_code`, chưa quá hạn theo metadata. Không refresh token, không sửa cache đăng nhập. Kết quả đã loại token và định danh nằm trong `desktopClaudeCheck` của báo cáo JSON.

- Hai lần `GET https://api.anthropic.com/api/oauth/usage` bằng token Desktop đều trả **403**, kể cả khi bổ sung User-Agent của Claude Code đã cài.
- Server trả `permission_error`: `OAuth authentication is currently not allowed for this organization.`
- Đây là lỗi quyền ở tổ chức được token đại diện, không phải kết quả 401 của token router cũ. Việc app Desktop vẫn làm việc không chứng minh endpoint quota OAuth này được tổ chức cho phép.
- File phiên web `Network/Cookies` không đọc được, báo **EBUSY**. Chưa lấy cookie phiên, chưa gửi request có xác thực tới `claude.ai`; không đóng hoặc khởi động lại app.

Kết luận cập nhật: Claude Desktop có nguồn đăng nhập riêng đã được tìm thấy. Kết quả kiểm tra `.claude` trước đó chỉ áp dụng cho cấu hình Claude Code, không phản ánh toàn bộ phiên Claude Desktop. Chưa xác minh được quota thật qua endpoint OAuth cho tổ chức hiện có trong cache. Nguồn usage trong giao diện Desktop hoặc phiên web cần được khảo sát riêng.

## Rà lại sai sót trong cách kiểm tra

Theo yêu cầu rà lại, xác nhận các giới hạn và sai sót sau:

1. Ban đầu chỉ kiểm tra `.claude` và credential router cũ. Các nguồn này không đủ để đánh giá phiên Claude Desktop đang làm việc. Cấu hình lưu trên đĩa cũng không chứng minh tiến trình đang chạy dùng đúng các giá trị đó; môi trường hoặc cấu hình riêng của phiên có thể ghi đè.
2. Token chưa quá hạn trong cache chỉ là metadata, không tự chứng minh token được server chấp nhận hoặc có quyền đọc quota.
3. Chưa xác định request thực tế của màn hình Usage trong Desktop. HTTP 403 ở `/api/oauth/usage` chỉ chứng minh những token đã thử bị từ chối trên endpoint này, không chứng minh mọi đường lấy quota Desktop đều bị chặn.
4. Đã bỏ sót `plan-usage-history.json` trong lần kiểm tra Desktop đầu. File có 363 mẫu tại thời điểm rà lại, trong đó 360 mẫu không rỗng; mẫu quota không rỗng gần nhất là **22/09/2026 lúc 22:46:10, Asia/Saigon**. Các mẫu mới nhất ngày 11/10 không chứa số liệu quota. Không dùng lịch sử cũ làm quota hiện tại.

Đã kiểm tra nghi vấn chọn sai tổ chức: tổ chức của token đã thử khớp tổ chức trong mẫu usage mới nhất và mẫu không rỗng gần nhất của Desktop. Vì vậy chưa có bằng chứng cho lỗi chọn sai tổ chức; vẫn chưa đối chiếu trực tiếp với tổ chức đang chọn trong giao diện app.

Lượt quota thứ ba dùng token Desktop chỉ có scope `user:profile`, khớp tài khoản và tổ chức trong lịch sử. Kết quả vẫn **403** với cùng thông báo từ server. Dừng thử endpoint OAuth này sau ba lượt. Không phát hiện được lỗi chọn loại token làm thay đổi kết quả, và không suy rộng kết quả đó sang phiên web hoặc giao diện Desktop.

Kết luận sau rà lại: vị trí cache OAuth đã tìm là đúng và các phản hồi 403 là có thật. Sai sót chính là khảo sát nguồn phiên chưa đầy đủ và diễn đạt tình trạng đăng nhập/quota rộng hơn bằng chứng. Cần xác định luồng request và xác thực thực tế của màn hình Usage trước khi chốt khả năng tích hợp Desktop.

## Thử lại Antigravity bằng app đang chạy

Ngày 11/10/2026 lúc 11:36:54, Asia/Saigon, đã đọc được quota của **Antigravity Desktop 2.11.0** qua language server đang chạy. App cài tại `C:\Users\mdbre\AppData\Local\Programs\antigravity`; log tại `C:\Users\mdbre\AppData\Roaming\Antigravity\logs\language_server.log` cung cấp cổng loopback hiện tại. Đây là nguồn riêng với token OAuth cũ đã nhập vào OmniRoute.

Đã đọc mã client `/main.js` do app phục vụ, xác định đúng service từ descriptor protobuf và gọi:

`POST /exa.language_server_pb.LanguageServerService/RetrieveUserQuotaSummary`

Request dùng `{"forceRefresh":false}` và header `X-Codeium-Csrf-Token` lấy từ cấu hình trang loopback. CSRF chỉ giữ trong bộ nhớ, chỉ gửi tới loopback; không trích OAuth token, không gọi model hoặc thay cấu hình đăng nhập.

**Kết quả HTTP 200:**

| Nhóm | Bucket | Quota tuần còn lại | Reset, Asia/Saigon |
| --- | --- | --- | --- |
| Gemini Models | `gemini-weekly` | **100%** (`remainingFraction=1`) | 18/10/2026, 11:30:42 |
| Claude and GPT models | `3p-weekly` | **100%** (`remainingFraction=1`) | 18/10/2026, 11:30:42 |

Hai nhóm và tỷ lệ khớp ảnh Models & Usage người dùng cung cấp, trong đó giao diện ghi Antigravity Starter Quota. Request đọc summary của app với `forceRefresh=false`, giống luồng UI đã đọc trong mã client; chưa độc lập xác minh một lần fetch mới trực tiếp tới Google hoặc khả năng đọc khi app không chạy.

Kết luận cập nhật: **đã xác minh một nguồn quota Antigravity phù hợp để tích hợp UsagePin khi app đang chạy**. Kết quả 401 trước chỉ áp dụng cho token router cũ, không phản ánh phiên Antigravity hiện tại. Kết quả này bổ sung nguồn quota có xác thực bên cạnh Cursor; chưa sửa mã tích hợp UsagePin. Dữ liệu đã loại credential nằm trong `antigravityNativeCheck` của báo cáo JSON.
