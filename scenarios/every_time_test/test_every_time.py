import os
import datetime
import time

last_run = datetime.datetime.now() - datetime.timedelta(minutes=10)

command = "erc20_processor --version"
print(command)
os.system(command)

while True:
    time.sleep(1)
    now = datetime.datetime.now()
    if now - last_run > datetime.timedelta(minutes=10):
        last_run = now
        command = f"erc20_processor generate --random-receivers -n 1 -a"
        print(command)
        os.system(command)

        command = f"erc20_processor run"
        print(command)
        os.system(command)
    else:
        print(f"Waiting for {now - last_run}")
