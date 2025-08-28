# --- 监听器回调函数 ---
import collections
import threading
from collections import defaultdict
from datetime import datetime

from pynput import keyboard, mouse

from src.monitor.maps import VK_MAP
from src.setting import SAVE_THRESHOLD


class MonitorListen:
    base_counts_today: dict
    incremental_counts = collections.defaultdict(int)
    total_clicks_since_save = 0

    def __init__(self, db):
        self.db = db
        self.data_lock = threading.Lock()
        self.init_data()

    def init_data(self):
        print("应用启动中...")
        # 初始化时加载当天的历史数据到内存
        today_str = datetime.now().strftime('%Y-%m-%d')
        self.base_counts_today = defaultdict(int, self.db.get_stats_for_day(today_str))
        if self.base_counts_today:
            print(f"成功从数据库加载了 {today_str} 的基础数据。")
        else:
            print(f"数据库中没有找到 {today_str} 的数据，从零开始。")

    def start(self):
        keyboard_listener = keyboard.Listener(on_release=self.on_release)
        mouse_listener = mouse.Listener(on_click=self.on_click)

        keyboard_thread = threading.Thread(target=keyboard_listener.start, daemon=True)
        mouse_thread = threading.Thread(target=mouse_listener.start, daemon=True)

        keyboard_thread.start()
        mouse_thread.start()
        print("键盘和鼠标监听器已启动。")

    def on_release(self, key):
        key_name = self.get_key_name(key)
        self.handle_event(key_name)

    def on_click(self, x, y, button, pressed):
        if pressed:
            button_name = f"mouse_{str(button).replace('Button.', '')}"
            self.handle_event(button_name)

    def handle_event(self, key_name: str):
        """统一处理键盘和鼠标事件"""
        if not key_name:
            return

        with self.data_lock:
            self.incremental_counts[key_name] += 1
            self.total_clicks_since_save += 1

            if self.total_clicks_since_save >= SAVE_THRESHOLD:
                self.save_to_db_locked()

    def get_key_name(self, key):
        if hasattr(key, 'vk'):
            key_name = VK_MAP.get(key.vk)
        elif (key_code := getattr(key, "_value_")) and key_code.vk in VK_MAP:
            key_name = VK_MAP.get(key._value_.vk)
        elif hasattr(key, '_name_'):
            key_name = key._name_
        else:
            try:
                key_name = key.char.lower() if key.char else ''
            except AttributeError:
                key_name = str(key).replace('Key.', '')
        return key_name

    def save_to_db_locked(self):
        """
        合并内存中的增量数据并保存到数据库。
        注意：此函数假定调用它的上下文已经持有了 data_lock。
        """

        if not self.incremental_counts:
            return

        # 1. 合并基础数据和增量数据
        total_today_counts = self.base_counts_today.copy()
        for key, value in self.incremental_counts.items():
            total_today_counts[key] = total_today_counts.get(key, 0) + value

        # 2. 将合并后的总数据存回数据库
        self.db.upsert_day_stats(datetime.now().strftime('%Y-%m-%d'), total_today_counts)

        # 3. 更新内存中的基础数据，并清空增量计数器
        self.base_counts_today = total_today_counts
        self.incremental_counts.clear()
        self.total_clicks_since_save = 0
        print(f"数据已合并并保存到数据库。当前时间: {datetime.now()}")

    def get_keycounts(self):
        with self.data_lock:
            # 实时合并基础数据和增量数据以供显示
            total_counts = self.base_counts_today.copy()
            for key, value in self.incremental_counts.items():
                total_counts[key] = total_counts.get(key, 0) + value
            return total_counts
