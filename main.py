from datetime import datetime

import uvicorn
from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware

from src.database import Database
from src.monitor import Monitor
from src.setting import CONFIG

# --- 核心逻辑 ---
db = Database(CONFIG.db_file)
monitor = Monitor(db)
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


# --- 主程序入口 ---
if __name__ == '__main__':
    print("全功能键盘鼠标记录器后端启动中...")
    print(f"每 {CONFIG.save_threshold} 次点击将自动保存数据到 {CONFIG.db_file}")
    print("在浏览器中打开 index.html 文件以查看。")
    uvicorn.run(app, host="0.0.0.0", port=CONFIG.port, log_config=None)
