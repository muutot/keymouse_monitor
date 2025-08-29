# --- 监听器回调函数 ---
import collections
import threading
from collections import defaultdict
from datetime import datetime
import time

from pynput import keyboard, mouse

from src.monitor.maps import VK_MAP
from src.setting import CONFIG

# 新增导入
from apscheduler.schedulers.background import BackgroundScheduler


class MonitorListen:
    base_counts_today: dict
    incremental_counts = collections.defaultdict(int)
    total_clicks_since_save = 0

    def __init__(self, db):
        self.db = db
        self.data_lock = threading.Lock()
        self.init_data()
        self.scheduler = BackgroundScheduler()
        self.scheduler.add_job(self.save_to_db_locked_with_lock, 'interval', minutes=1)
        self.scheduler.start()

    # ... 现有方法保持不变 ...

    def start(self):
        keyboard_listener = keyboard.Listener(on_release=self.on_release)
        mouse_listener = mouse.Listener(on_click=self.on_click)

        keyboard_thread = threading.Thread(target=keyboard_listener.start, daemon=True)
        mouse_thread = threading.Thread(target=mouse_listener.start, daemon=True)

        keyboard_thread.start()
        mouse_thread.start()
        print("键盘和鼠标监听器已启动。")

    def save_to_db_locked_with_lock(self):
        with self.data_lock:
            self.save_to_db_locked()

    # ... 其他现有方法 ...