class Timer:

    def __init__(self):
        self.timers = {}
        self.func_times = {}



    def timer_cycle(self, interval, function, *args, **kwargs):
