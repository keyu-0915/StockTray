FROM python:3.11-slim

RUN pip install --no-cache-dir --disable-pip-version-check futu-api

COPY verify.py /verify.py

CMD ["python", "/verify.py"]
