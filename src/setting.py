# --- 全局变量和配置 ---
import json


class Config:
    db_file: str
    save_threshold: int

    def __init__(self):
        self.init_config()

    def init_config(self):
        with open("./config.json", 'r') as f:
            for key, value in json.load(f).items():
                setattr(self, key, value)
        print(self.__dict__)


CONFIG = Config()
