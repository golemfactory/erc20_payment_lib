FROM python:3.10
RUN apt-get update
# install common helpful tools
RUN apt-get install -y curl vim jq net-tools htop iptables build-essential iputils-ping iproute2 dnsutils ncat tcpdump
# install helpful python libraries
RUN pip install requests websockets aiohttp

RUN update-ca-certificates