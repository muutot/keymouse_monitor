[x] 程序结束前保存一份当前内存数据到数据库
[x] 刷新实时数据后全为0 这个需要修复一下, 点击对应按钮之后就好了
[x] 查询历史应该是两边都闭的情况（MongoDB $gte/$lte 和 SQLite BETWEEN 本来就是闭区间）
[x] 刷新页面之后又闪烁需要修复
[x] 扁平化存储结构 (date, key, count) — 消除 $unwind，支持导入导出新旧两种格式
[x] 前台导出按钮增加格式切换（format=nested|flat）
[ ] 支持config与exe相对同级目录 而不是相对当前目录
[ ] 支持log文件与exe相对同级目录 而不是相对当前目录




