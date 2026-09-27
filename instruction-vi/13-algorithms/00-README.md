# Algorithms

Các cấu trúc dữ liệu và thuật toán mà phần còn lại của handbook được xây
trên đó. Routing cần trie/radix tree, WAF cần Aho-Corasick, rate limiting
cần token bucket, caching cần LRU/TinyLFU, load balancing cần smooth
WRR/consistent hash/Maglev, DDoS mitigation cần count-min-sketch/HyperLogLog.
Thư mục này nói về bản thân thuật toán; [`05-http-stack/`](../05-http-stack), [`06-proxy/`](../06-proxy), và
[`07-security/`](../07-security) nói về cách nó được lắp vào proxy và trỏ ngược lại đây để
đào sâu.

## Đã viết

Phần lớn các file này backing trực tiếp một bài tập cụ thể trong [`labs/`](../../labs);
một vài file ([`dfa.md`](dfa.md), [`fsm.md`](fsm.md), [`skiplist.md`](skiplist.md), [`priority-queue.md`](priority-queue.md)) là
nền tảng tổng quát không có lab riêng — file của chính chúng nói rõ nó áp
dụng ở đâu thay vào đó:

- [`smooth-wrr.md`](smooth-wrr.md) — thuật toán current-weight của nginx, effective weight, phản ứng khi lỗi thụ động
- [`consistent-hash.md`](consistent-hash.md) — cách sizing virtual node, bài toán cân bằng tải, bounded loads
- [`rendezvous-hash.md`](rendezvous-hash.md) — HRW: gián đoạn tối thiểu mà không cần ring hay shared state
- [`maglev.md`](maglev.md) — bảng lookup O(1), cách xây permutation, vòng lặp population
- [`lru.md`](lru.md) — LRU dùng arena, vấn đề mỗi lần đọc cũng là một lần ghi lock, CLOCK, khả năng chống scan
- [`token-bucket.md`](token-bucket.md) — refill lazy, GCRA, cập nhật lock-free, chặn key tăng vô hạn
- [`aho-corasick.md`](aho-corasick.md) — trie + failure link, output link, prefilter theo literal
- [`count-min-sketch.md`](count-min-sketch.md) — đếm tần suất xấp xỉ trong bộ nhớ cố định, error bound, decay
- [`regex-engine.md`](regex-engine.md) — backtracking so với automata, ReDoS, lazy DFA, `RegexSet`
- [`slab.md`](slab.md) — intrusive free list, generational handle, vấn đề shrink
- [`sliding-window.md`](sliding-window.md) — bug boundary-burst của fixed window, sliding log, xấp xỉ two-counter
- [`leaky-bucket.md`](leaky-bucket.md) — dạng meter (ảnh gương của token bucket) so với dạng queue (làm mượt output)
- [`lfu.md`](lfu.md) — cấu trúc frequency-bucket O(1), vấn đề stale winner, aging
- [`arc.md`](arc.md) — T1/T2/B1/B2, điều chỉnh tỉ lệ recency/frequency từ các lần hit ở ghost-list
- [`tinylfu.md`](tinylfu.md) — admission thay vì eviction, tần suất bằng count-min-sketch, doorkeeper, cửa sổ LRU của W-TinyLFU
- [`dfa.md`](dfa.md) — matching theo bảng, subset construction, minimization
- [`fsm.md`](fsm.md) — máy Mealy/Moore, enum+match so với typestate pattern
- [`trie.md`](trie.md) — tra cứu prefix theo segment, được [`05-http-stack/03-router.md`](../05-http-stack/03-router.md) dùng
- [`radix-tree.md`](radix-tree.md) — trie nén, cách tách longest-common-prefix, thứ mà router production thật dùng
- [`ring-buffer.md`](ring-buffer.md) — circular buffer kích thước cố định, logging SPSC lock-free, được [`08-observability/01-logging.md`](../08-observability/01-logging.md) dùng
- [`heap.md`](heap.md) — binary heap dựa trên mảng, decrease-key qua lazy deletion hoặc indexed heap
- [`priority-queue.md`](priority-queue.md) — timer wheel, vì sao `tokio::time` không dùng heap cho timeout
- [`hashmap.md`](hashmap.md) — chaining so với open addressing, SwissTable, hash flooding khi key do attacker kiểm soát
- [`skiplist.md`](skiplist.md) — level xác suất, vì sao lock-free skip list phổ biến ở nơi lock-free tree thì không
- [`bloom-filter.md`](bloom-filter.md) — kiểm tra membership không có false negative, cách sizing, dùng làm doorkeeper của TinyLFU
- [`hyperloglog.md`](hyperloglog.md) — ước lượng cardinality trong bộ nhớ cố định, mergeable giữa các shard, được [`07-security/09-ddos.md`](../07-security/09-ddos.md) dùng
