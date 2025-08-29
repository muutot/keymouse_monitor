# --- 全局变量和配置 ---
import json
import os.path


class Config:
    config_path = "./config.json"

    db_file: str = "monitor.sqlite"
    save_threshold: int = 20
    port: int = 8080

    def __init__(self):
        self.init_config()

    def init_config(self):
        if not os.path.exists(self.config_path):
            print("不存在Config, 读取默认配置".center(80, "="))
            self.show_config()
            return
        with open(self.config_path, 'r') as f:
            for key, value in json.load(f).items():
                setattr(self, key, value)
        print("存在Config, 数据读取中".center(80, "="))
        self.show_config()

    def show_config(self):
        for key, value in self.__dict__.items():
            print(f"{key}: {value}")


CONFIG = Config()
