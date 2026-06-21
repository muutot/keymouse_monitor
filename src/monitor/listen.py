# --- 监听器回调函数 ---
import threading
import weakref

from pynput import keyboard, mouse

from src.monitor.maps import get_key_name
from src.type_model import MonitorT


class MonitorListen:

    def __init__(self, parent):
        self.parent: MonitorT = weakref.proxy(parent)

    @property
    def db(self):
        return self.parent.db

    def start(self):
        keyboard_listener = keyboard.Listener(on_release=self.on_release)
        mouse_listener = mouse.Listener(on_click=self.on_click, on_scroll=self.on_scroll)

        keyboard_thread = threading.Thread(target=keyboard_listener.start, daemon=True)
        mouse_thread = threading.Thread(target=mouse_listener.start, daemon=True)

        keyboard_thread.start()
        mouse_thread.start()
        print("键盘和鼠标监听器已启动。")

    def on_release(self, key):
        key_name = get_key_name(key)
        self.handle_event(key_name)

    def on_click(self, x, y, button, pressed):
        if not pressed:
            return
        button_name = f"mouse_{str(button).replace('Button.', '')}"
        self.handle_event(button_name)

    def on_scroll(self, x, y, dx, dy):
        if dy > 0:
            self.handle_event("mouse_scroll_up")
        elif dy < 0:
            self.handle_event("mouse_scroll_down")
        if dx > 0:
            self.handle_event("mouse_scroll_right")
        elif dx < 0:
            self.handle_event("mouse_scroll_left")

    def handle_event(self, key_name: str):
        """统一处理键盘和鼠标事件"""
        if not key_name:
            return
        self.parent.data.increase_count(key_name)
