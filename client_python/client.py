import os

os.environ['TF_CPP_MIN_LOG_LEVEL'] = '3'

import tensorflow as tf
import json
import numpy as np
import zmq
import time
tf.random.set_seed(42)


context = zmq.Context()
socket = context.socket(zmq.REQ)
socket.connect("tcp://localhost:" + os.getenv("PORT"))
clients = os.getenv("CLIENTS")

nn_model = tf.keras.models.Sequential([
  tf.keras.layers.Flatten(input_shape=(28, 28)),
  tf.keras.layers.Dense(128, activation='relu'),
  tf.keras.layers.Dense(10)
])
nn_model.compile(
    optimizer=tf.keras.optimizers.Adam(0.001),
    loss=tf.keras.losses.SparseCategoricalCrossentropy(from_logits=True),
    metrics=[tf.keras.metrics.SparseCategoricalAccuracy()],
)

X = np.load("/mnt/X_" + clients + ".npy")
Y = np.load("/mnt/Y_" + clients + ".npy")

X_test = np.load("/mnt/X_test.npy")
Y_test = np.load("/mnt/Y_test.npy")

X = X.astype('float32')
X_test = X_test.astype('float32')
X /= 255
X_test /= 255


socket.send_string("1")
socket.recv_string()
accuracies_local = []
accuracies_global = []
start = time.time()
while True:
    nn_model.fit(X, Y, epochs=5, batch_size=32, steps_per_epoch=3)
    loss, accuracy = nn_model.evaluate(X_test, Y_test)
    accuracies_local.append(accuracy)
    arr = nn_model.get_weights()
    f = [w.tolist() for w in arr]
    with open("/mnt/simple.json", "w") as outfile:
        outfile.write(json.dumps(f))

    socket.send_string("1")
    res = socket.recv_string()
    
    with open("/mnt/models.json", "r") as infile:
        models = json.load(infile)
    models = [np.array(json.loads(model), dtype=object) for model in models]
    newGM = []
    for model in zip(*models):
        newGM.append(np.array(model).mean(axis=0))

    nn_model.set_weights(newGM)
    loss, accuracy = nn_model.evaluate(X_test, Y_test)
    accuracies_global.append(accuracy)
    if res == "stop":
        break
end = time.time()
elapsed = end - start
with open("/mnt/latency_python_" + clients + ".txt", "a") as f:
    f.write(str(elapsed) + "\n")
with open("/mnt/acc_local_python_" + clients + ".txt", "a") as f:
    json.dump(accuracies_local, f)
with open("/mnt/acc_global_python_" + clients + ".txt", "a") as f:
    json.dump(accuracies_global, f)

#loss, accuracy = nn_model.evaluate(X_test, Y_test)

socket.close()