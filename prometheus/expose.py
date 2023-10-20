from flask import Flask

app = Flask(__name__)


@app.route("/metrics", methods=['GET'])
def getfile():
    with open("metrics.txt", "r+") as f:
        data = f.read()
        return data


if __name__ == '__main__':
    app.run(host='localhost')
