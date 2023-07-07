FROM ethereum/client-go as builder

FROM nikolaik/python-nodejs:python3.10-nodejs18
RUN apt-get update
RUN apt-get install -y vim sqlite3
RUN pip install web3 python-dotenv

WORKDIR /runtime/contracts
COPY contracts/package.json .
COPY contracts/package-lock.json .
RUN npm install

#copy geth from client-go
COPY --from=builder /usr/local/bin/geth /usr/local/bin/

#copy contracts and compile
COPY contracts/*.js ./
COPY contracts/contracts ./contracts
COPY contracts/scripts ./scripts
RUN npm run compile

#copy python scripts
WORKDIR /runtime
COPY *.py .
