import asyncio
import json
import sys
from datetime import datetime

import uvicorn
from fastapi import FastAPI, HTTPException, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import StreamingResponse

from src.database import Database
from src.monitor import Monitor
from src.setting import CONFIG

# --- 核心逻辑 ---
db = Database(CONFIG.db_file)
monitor = Monitor(db)

# SSE 通知机制: 存储所有活跃的 SSE 连接事件
_sse_events: set[asyncio.Event] = set()
_loop: asyncio.AbstractEventLoop = None


def _notify_sse():
    """由后台线程(键盘/鼠标事件)调用, 通知所有 SSE 连接有新数据"""
    for event in _sse_events.copy():
        _loop.call_soon_threadsafe(event.set)


# --- FastAPI Web 服务器逻辑 ---
app = FastAPI()

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


@app.on_event("startup")
async def startup_event():
    """FastAPI 启动时执行"""
    global _loop
    _loop = asyncio.get_running_loop()
    monitor.data.on_increase = _notify_sse
    monitor.start()


@app.get("/keycounts")
def get_keycounts():
    """ API 端点: 返回当前的实时数据 (基础数据 + 增量数据) """
    return monitor.get_keycounts()


@app.get("/history")
def get_history(start: str, end: str):
    """ API 端点: 根据日期区间查询并合并历史数据 """
    try:
        datetime.strptime(start, '%Y-%m-%d')
        datetime.strptime(end, '%Y-%m-%d')
    except ValueError:
        raise HTTPException(status_code=400, detail="日期格式无效，请使用 YYYY-MM-DD 格式。")

    return db.get_stats_for_range(start, end)


@app.get("/events")
async def sse_handler(request: Request):
    """SSE 端点: 按键/鼠标事件触发时推送数据, 而非定时轮询"""
    async def event_stream():
        my_event = asyncio.Event()
        _sse_events.add(my_event)
        try:
            # 立即发送一次初始数据
            yield f"data: {json.dumps(monitor.get_keycounts())}\n\n"
            while True:
                if await request.is_disconnected():
                    break
                try:
                    await asyncio.wait_for(my_event.wait(), timeout=30)
                except asyncio.TimeoutError:
                    continue
                my_event.clear()
                data = json.dumps(monitor.get_keycounts())
                yield f"data: {data}\n\n"
        finally:
            _sse_events.discard(my_event)

    return StreamingResponse(event_stream(), media_type="text/event-stream")


# --- 主程序入口 ---
if __name__ == '__main__':
    show_console = any(arg in sys.argv for arg in ('--console', '-c'))

    print("全功能键盘鼠标记录器后端启动中...")
    if show_console:
        print("Console mode enabled")
    print(f"每 {CONFIG.save_threshold} 次点击将自动保存数据到 {CONFIG.db_file}")
    print("在浏览器中打开 index.html 文件以查看。")
    uvicorn.run(app, host="0.0.0.0", port=CONFIG.port, log_config=None)
