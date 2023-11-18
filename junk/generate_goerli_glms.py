import os

command = "cargo run -- generate-key -n 1 > .env"
print(command)
os.system(command)

command = "cargo run -- get-dev-eth -c goerli"
print(command)
os.system(command)

while True:
    command = "cargo run -- mint-test-tokens -c goerli"
    print(command)
    os.system(command)

    command = "cargo run -- run"
    print(command)
    os.system(command)

    command = "cargo run -- transfer -c goerli --token glm --amount 1000 --recipient 0xC596AEe002EBe98345cE3F967631AaF79cFBDF41"
    print(command)
    os.system(command)

    command = "cargo run -- run"
    print(command)
    os.system(command)
