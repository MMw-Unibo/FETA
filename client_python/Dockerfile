FROM python:3.8

WORKDIR /.

COPY requirements.txt requirements.txt

RUN pip3 install -r requirements.txt

COPY . .

CMD ["python3","-u","client.py"]


