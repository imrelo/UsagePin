# Kế hoạch phát triển UsagePin

Ngày khảo sát: 09/10/2026. Phạm vi: lập kế hoạch dựa trên mã nguồn hiện tại; chưa triển khai tính năng và chưa chạy kiểm thử ứng dụng.

## Mục tiêu và giả định

Mục tiêu đã được xác nhận: khảo sát UsagePin và lập kế hoạch tính năng có ưu tiên, giai đoạn và tiêu chí hoàn thành kiểm chứng được.

Giả định để lập kế hoạch: tiếp tục hướng sản phẩm thể hiện trong giao diện — widget Windows theo dõi hạn mức Codex cho một tài khoản. Bản đầu phục vụ người dùng đã cài Codex; một lập trình viên triển khai. Ước lượng bên dưới là ngày công sơ bộ, cần điều chỉnh sau khi kiểm chứng nguồn dữ liệu, không phải cam kết lịch phát hành.

Cách triển khai đơn giản nhất đề xuất: giữ widget hiện tại, bổ sung một nguồn dữ liệu Codex và Settings cần thiết. Chưa mở rộng thành dashboard, dịch vụ đồng bộ hoặc nền tảng theo dõi nhiều nhà cung cấp.

## Hiện trạng đã đọc từ mã nguồn

| Thành phần | Đã có | Khoảng trống liên quan đến kế hoạch |
| --- | --- | --- |
| Widget | Cửa sổ trong suốt, luôn nổi, kéo vị trí, đổi kích thước giữ tỷ lệ | Chưa lưu kích thước, chưa kiểm tra vị trí phục hồi khi thay màn hình |
| Hiển thị usage | Hai thanh phần trăm và thời điểm reset | `frontend/App.tsx` dùng `usageRows` cố định và nhãn `SAMPLE DATA`; chưa có nguồn dữ liệu thật |
| Settings | Độ trong suốt 20–100%, chuyển chế độ chuột | Chưa có trạng thái kết nối, cập nhật dữ liệu, khởi động cùng Windows, cảnh báo |
| Lưu cấu hình | Vị trí và độ trong suốt trong localStorage | Click-through chỉ lưu trong bộ nhớ Rust; cần quy định rõ trạng thái khi khởi động |
| Tray | Đổi click-through, thoát app | Chưa có mở Settings, ẩn/hiện, làm mới, khôi phục widget |
| Kiểm thử | Có ba test Rust cho đổi kích thước | CI hiện chạy frontend build và `cargo check`, chưa chạy các test này |
| Phát hành | Có workflow Windows và cấu hình đóng gói Store | Chưa có updater, metadata và bộ nhận diện hoàn chỉnh; tài liệu xuất bản còn các bước cần tài khoản/publisher |

Ghi chú: hàm mẫu `greet` trong `backend/src/lib.rs` được đăng ký nhưng không được frontend hiện tại gọi. Đây là mã tồn tại trước; kế hoạch này không yêu cầu xóa hoặc dọn mã lân cận.

## Cơ sở chọn nguồn dữ liệu

Tài liệu chính thức mô tả `account/rateLimits/read`, sự kiện `account/rateLimits/updated`, các trường `usedPercent`, `windowDurationMins`, `resetsAt` và `rateLimitsByLimitId`. Nguồn: [Codex App Server — tài liệu OpenAI](https://learn.chatgpt.com/docs/app-server).

Đề xuất thử dùng Codex app-server qua stdio trong backend Rust. Đây là lựa chọn thiết kế dựa trên tài liệu, chưa được kiểm chứng trên máy này. Tool đọc usage có sẵn trong cuộc chat không được xem là API mà UsagePin có thể sử dụng khi phát hành.

Phải kiểm chứng phiên bản CLI, cách sử dụng phiên đăng nhập và hạn mức thực sự tài khoản trả về. Không mặc định mọi tài khoản có hai cửa sổ 5 giờ/tuần. Không dùng API usage tính phí làm dữ liệu thay thế cho hạn mức Codex.

## Các giai đoạn theo thứ tự thực hiện

### 0. Kiểm chứng nguồn dữ liệu — P0, 1–2 ngày công

Phạm vi:

- Xác định phiên bản Codex CLI và khả năng chạy app-server trên Windows.
- Thử luồng initialize, kiểm tra tài khoản và đọc hạn mức; chỉ cần thao tác đọc, không tạo lượt suy luận.
- Xác minh tài khoản có đúng với tài khoản người dùng muốn theo dõi; không giả định CLI và app desktop luôn dùng chung tài khoản.
- Kiểm tra cập nhật bằng sự kiện, đọc lại định kỳ và lỗi mất kết nối/hết phiên đăng nhập.
- Ghi nhận phản hồi mẫu đã loại thông tin nhận diện và bí mật để làm fixture kiểm thử.
- Đánh giá chi phí chạy tiến trình app-server và cách dừng tiến trình khi UsagePin thoát.

Tiêu chí hoàn thành: đọc được phần trăm đã dùng và thời điểm reset từ tài khoản thật, đối chiếu cùng tài khoản tại thời điểm gần nhau; xác định được luồng khi chưa đăng nhập, CLI không có hoặc không tương thích. Có quyết định nguồn dữ liệu và phiên bản hỗ trợ được ghi lại.

Điểm chặn: nếu không chứng minh được nguồn dữ liệu hoạt động, dừng phần tích hợp và báo kết quả. Không coi dữ liệu demo là đạt tiêu chí, không tự chuyển sang đọc endpoint nội bộ hoặc scraping để đổi mục tiêu.

### 1. Widget dữ liệu thật — P0, 3–5 ngày công

Phụ thuộc: giai đoạn 0 đạt tiêu chí.

Phạm vi:

- Backend quản lý một kết nối nguồn dữ liệu, cung cấp snapshot và phát sự kiện sang frontend.
- Thay `usageRows` bằng trạng thái dữ liệu; phần trăm còn lại = `clamp(100 - usedPercent, 0, 100)` khi giá trị đầu vào hợp lệ.
- Ưu tiên dữ liệu theo `rateLimitsByLimitId` nếu có; hỗ trợ phản hồi một bucket cũ. Bản đầu chọn bucket Codex và thể hiện rõ nguồn đang theo dõi.
- Sinh nhãn theo thời lượng thực tế; giá trị thiếu hiển thị “Không có dữ liệu”, không chuyển thành 0% hoặc 100%.
- Hiển thị giờ reset theo múi giờ máy, đếm ngược và thời điểm cập nhật gần nhất.
- Có trạng thái đang kết nối, chưa đăng nhập, cập nhật thành công, dữ liệu cũ, lỗi và nguồn không được hỗ trợ.
- Làm mới thủ công; đề xuất đọc lại mỗi 60–120 giây, ưu tiên sự kiện khi có, không gửi nhiều yêu cầu làm mới chồng nhau. Chốt chu kỳ sau đo thực tế.
- Khi đến giờ reset, đọc lại nguồn; không tự đặt phần trăm còn lại thành 100%.
- Hướng dẫn ngắn trong Settings cho trường hợp thiếu CLI hoặc cần đăng nhập; chưa xây hệ thống tài khoản riêng.

Tiêu chí hoàn thành: fixture đã dùng 25% hiển thị còn 75%; thiếu cửa sổ phụ vẫn hiển thị đúng; reset hiển thị đúng múi giờ; nguồn thay đổi thì widget cập nhật; lỗi không bị hiển thị như dữ liệu mới. Chỉ dữ liệu demo mới có nhãn demo.

### 2. Hoàn thiện sử dụng hằng ngày — P1, 2–4 ngày công

Phụ thuộc: widget dữ liệu thật hoạt động.

Phạm vi:

- Tray có mở Settings, ẩn/hiện widget, làm mới, đưa widget về vùng nhìn thấy và thoát.
- Tùy chọn khởi động cùng Windows; chỉ một instance app, mở lần nữa đưa người dùng về instance hiện có.
- Lưu kích thước và màn hình/vị trí; phục hồi trong vùng làm việc còn khả dụng khi tháo màn hình hoặc đổi DPI.
- Quy định và lưu tùy chọn click-through; luôn có đường khôi phục tương tác từ tray.
- Thử hotkey toàn hệ thống để bật/tắt tương tác; nếu phím bị chiếm, báo lỗi rõ và vẫn dùng được tray.
- Dữ liệu cuối cùng có thể được lưu cục bộ kèm tài khoản/nguồn và thời gian, luôn đánh dấu là cũ đến khi xác minh lại. Không dùng cache của tài khoản trước cho tài khoản mới.
- Kết nối lại sau sleep/wake, giảm tần suất retry khi lỗi kéo dài, dừng tiến trình nguồn khi app thoát.

Tiêu chí hoàn thành: mở app hai lần vẫn chỉ có một widget; khởi động lại giữ cấu hình; tháo màn hình không làm mất widget; click-through luôn phục hồi được qua tray; sleep/wake và mất mạng không gây vòng lặp retry liên tục; thoát app không để lại tiến trình nguồn do UsagePin tạo.

### 3. Cảnh báo hạn mức — P1, 2–3 ngày công

Phụ thuộc: trạng thái dữ liệu và khôi phục kết nối đáng tin cậy.

Phạm vi:

- Đổi màu thanh theo mức còn lại, kèm nội dung để không chỉ dựa vào màu.
- Tùy chọn cảnh báo Windows khi còn dưới 20%, dưới 10% hoặc hết hạn mức.
- Bật/tắt cảnh báo, chỉnh ngưỡng và nhắc khi hạn mức được nguồn xác nhận đã reset.
- Lưu trạng thái cảnh báo theo tài khoản, bucket và cửa sổ reset để tránh lặp sau restart.
- Không cảnh báo dựa trên dữ liệu demo, lỗi hoặc dữ liệu cũ.

Tiêu chí hoàn thành: chuỗi còn 25% → 19% → 18% chỉ tạo một cảnh báo ngưỡng 20%; restart không phát lại; cửa sổ mới cho phép cảnh báo mới; tắt cảnh báo được tôn trọng; thiếu quyền notification có thông báo xử lý phù hợp.

### 4. Beta và phát hành — P1, 3–5 ngày công

Phụ thuộc: các tiêu chí giai đoạn 1–3 đạt.

Phạm vi:

- About có phiên bản, nguồn dữ liệu và thông tin hỗ trợ; icon và metadata UsagePin hoàn chỉnh.
- Chọn kênh phát hành Windows và cập nhật tài liệu cài đặt, đăng nhập, khôi phục widget, gỡ ứng dụng.
- Kiểm tra yêu cầu hiện hành của kênh được chọn; hoàn thiện installer, ký và cơ chế cập nhật phù hợp. Tài khoản publisher/chứng thư là phụ thuộc ngoài phần code.
- Chạy các test Rust hiện có trong CI; thêm test cho chuyển đổi quota, dữ liệu thiếu, trạng thái dữ liệu cũ, chống trùng refresh và chống lặp cảnh báo.
- Thử trên máy Windows sạch, nhiều màn hình và các mức DPI 100%, 125%, 150%, 200%; thử cài mới, nâng cấp giữ cấu hình và gỡ app.
- Kiểm tra log không chứa token hay thông tin đăng nhập; đo CPU/RAM khi idle và mất kết nối. Ghi baseline rồi chốt ngưỡng phát hành, chưa đặt con số khi chưa đo.

Tiêu chí hoàn thành: CI build và test đều xanh; cài/nâng cấp/gỡ hoạt động trên máy thử; không mất widget trong ma trận màn hình; log đã kiểm tra; cơ chế cập nhật đã thử; phụ thuộc phát hành còn thiếu được ghi rõ.

Ước lượng tổng cho giai đoạn 0–4: 11–19 ngày công sau khi các giả định phù hợp. Chưa tính thời gian xử lý tài khoản phát hành, chứng thư, xét duyệt Store hoặc thay đổi lớn trong nguồn dữ liệu.

## Các tính năng P2 để đánh giá sau beta

| Tính năng | Lý do xem xét | Điều kiện trước khi đưa vào sprint |
| --- | --- | --- |
| Chế độ compact và số liệu trong tray tooltip | Giảm diện tích chiếm màn hình | Người dùng beta cần và vẫn đọc rõ ở DPI cao |
| Tiếng Việt/Anh, định dạng giờ | Dễ dùng cho nhiều nhóm người dùng | Gom chuỗi giao diện khi thực sự thêm bản địa hóa |
| Chọn bucket hạn mức | Hỗ trợ tài khoản có nhiều loại hạn mức | Nguồn trả nhiều bucket và có nhu cầu rõ ràng |
| Lịch sử 7/30 ngày | Xem xu hướng sử dụng | Quyết định dùng số liệu gốc hay snapshot cục bộ; ghi rõ khoảng trống dữ liệu |
| Nhiều tài khoản | Theo dõi tài khoản cá nhân/công việc | Chốt luồng đăng nhập, lưu bí mật và tách cache trước |
| Nhà cung cấp khác, macOS/Linux | Mở rộng thị trường | Có nhu cầu sau beta; phần resize hiện tại dùng API Windows |

## Phạm vi kỹ thuật cần thiết khi triển khai

- `backend/src/lib.rs`: đăng ký command, state và menu cần cho từng tính năng; bổ sung module nguồn usage/cấu hình/cảnh báo khi phần đó được triển khai.
- `frontend/App.tsx`: kết nối dữ liệu và các thao tác của widget/Settings; giữ giao diện hiện tại cho MVP.
- `frontend/App.css`: chỉ thêm kiểu cho trạng thái dữ liệu và cảnh báo phát sinh từ tính năng.
- `backend/src/resize.rs`: giữ logic hiện có; kiểm tra tương tác với phục hồi kích thước/DPI.
- Capability, dependency và CI: chỉ bổ sung quyền/thư viện/check cần cho tính năng được chọn.
- Backend giữ việc giao tiếp nguồn và bí mật; frontend chỉ nhận dữ liệu dùng hiển thị.

Không có công việc dọn code, đổi framework hoặc tái cấu trúc toàn ứng dụng trong kế hoạch. Mỗi lần triển khai phải xác nhận mục tiêu kiểm chứng được trước, giữ tối đa ba vòng sửa theo quy tắc làm việc; nếu cần đổi tiêu chí thì dừng và trao đổi.

## Bước triển khai đầu tiên đề xuất

Chọn riêng giai đoạn 0. Mục tiêu cho lượt triển khai đó: “UsagePin đọc được hạn mức và giờ reset thật của đúng tài khoản Codex trên Windows, đồng thời nhận diện được trường hợp chưa đăng nhập hoặc thiếu nguồn dữ liệu.” Xác nhận mục tiêu này trước khi bắt đầu code; toàn bộ roadmap chưa phải sự cho phép triển khai mọi giai đoạn.
